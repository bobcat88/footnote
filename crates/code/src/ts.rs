//! The tree-sitter base layer (CODE-GRAPH-DESIGN.md section 12): definitions, calls and imports parsed inside the binary for
//! every supported language, with no outside indexer and no build. A call is linked only when it resolves: to a definition in
//! the same file, through the file's imports (a project module or an outside package), or to a name defined once in the
//! project. An ambiguous call gets no link: precision over recall. Links made here are graded `resolved`, not `compiler`.
//! The output has the same shape as a SCIP index, so the graph, lookups and the map need nothing new.
use crate::scip::{DEFINITION, Document, GUESS, MACRO_ARG, Occurrence, Symbol, Unresolved};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Lang {
    Rust,
    Ts,
    Tsx,
    Js,
    Python,
    Java,
    Go,
}

fn lang_of(path: &str) -> Option<Lang> {
    Some(match path.rsplit('.').next()? {
        "rs" => Lang::Rust,
        "ts" | "mts" | "cts" => Lang::Ts,
        "tsx" => Lang::Tsx,
        "js" | "jsx" | "mjs" | "cjs" => Lang::Js,
        "py" => Lang::Python,
        "java" => Lang::Java,
        "go" => Lang::Go,
        _ => return None,
    })
}

fn grammar(l: Lang) -> (Language, &'static str, &'static str) {
    match l {
        Lang::Rust => (
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::TAGS_QUERY,
            "(use_declaration argument: (_) @imp)",
        ),
        Lang::Ts => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::TAGS_QUERY,
            JS_IMPORTS,
        ),
        Lang::Tsx => (
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            tree_sitter_typescript::TAGS_QUERY,
            JS_IMPORTS,
        ),
        Lang::Js => (
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::TAGS_QUERY,
            JS_IMPORTS,
        ),
        Lang::Python => (
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::TAGS_QUERY,
            "(import_statement) @imp (import_from_statement) @imp",
        ),
        Lang::Java => (
            tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::TAGS_QUERY,
            "(import_declaration) @imp",
        ),
        Lang::Go => (
            tree_sitter_go::LANGUAGE.into(),
            tree_sitter_go::TAGS_QUERY,
            "(import_spec) @imp",
        ),
    }
}

const JS_IMPORTS: &str = "(import_statement) @imp";
const RUST_PATH_CALLS: &str =
    "(call_expression function: (scoped_identifier name: (identifier) @name)) @reference.call";
const RUST_FIELDS: &str = "(struct_item name: (type_identifier) @s body: (field_declaration_list (field_declaration name: (field_identifier) @f type: (_) @t)))";

/// Folders that hold dependencies, builds or caches, never the project's own code.
const SKIP: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    ".next",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".gradle",
    "bin",
    "obj",
    "coverage",
];
const MAX_FILE: u64 = 1 << 20;
const MAX_FILES: usize = 20_000;

/// One definition: its name, kind (`function` or `type`), the file, the name's range and the whole item's range.
struct Def {
    name: String,
    kind: &'static str,
    file: usize,
    range: [i32; 4],
    span: [i32; 4],
    signature: String,
    /// Rust: the type whose `impl` (or trait) holds this function, and its declared return type with `Self` spelled out.
    owner: Option<String>,
    ret: Option<String>,
    /// Java: how many parameters, or `None` for a varargs method; tells overloads apart by argument count.
    arity: Option<usize>,
}

/// What a Rust method call is called on, as steps the method tables can follow: `self.store.open()?` is the impl's type,
/// then field `store`, then method `open`, then `?`.
#[derive(Clone, Debug)]
enum Base {
    Type(String),
    /// `Type::f()`: whatever `f` returns.
    Static(String, String),
}
#[derive(Clone, Debug)]
enum Step {
    Field(String),
    Call(String),
    Try,
}

/// One call: the called name, its qualifier (`os` in `os.path.join`, `self`, or empty) and where it is.
struct Call {
    name: String,
    qualifier: String,
    /// Called on a value (`obj.f()`), not through a path (`a::f()`) or bare: only `self`/`this` or an imported module may resolve it.
    member: bool,
    /// Rust member calls: the receiver, when it can be traced to a type.
    recv: Option<(Base, Vec<Step>)>,
    /// Java: how many arguments the call passes.
    args: Option<usize>,
    /// Java bare calls: the named classes around the call.
    within: Vec<String>,
    file: usize,
    range: [i32; 4],
    /// Found inside a Rust macro's arguments: linked with `scip::MACRO_ARG`, so a compiler graph can take it (#197).
    in_macro: bool,
}

/// Calls inside a Rust macro's token tree: an identifier right before a `(..)` group, bare (`run(..)`) or after a path
/// (`store::open(..)`, qualifier `store`). Method calls (`.f(..)`) are left out: no receiver type is known there.
fn macro_calls<'a>(mac: Node<'a>, src: &[u8]) -> Vec<(Node<'a>, String)> {
    let mut out = Vec::new();
    let mut c = mac.walk();
    let mut stack: Vec<Node> = mac
        .children(&mut c)
        .filter(|k| k.kind() == "token_tree")
        .collect();
    while let Some(tree) = stack.pop() {
        let mut c = tree.walk();
        let kids: Vec<Node> = tree.children(&mut c).collect();
        for (i, k) in kids.iter().enumerate() {
            if k.kind() == "token_tree" {
                stack.push(*k);
                continue;
            }
            let next_is_args = kids.get(i + 1).is_some_and(|n| {
                n.kind() == "token_tree" && n.utf8_text(src).is_ok_and(|t| t.starts_with('('))
            });
            if k.kind() != "identifier" || !next_is_args {
                continue;
            }
            let prev = |back: usize| i.checked_sub(back).and_then(|j| kids.get(j));
            let prev_text =
                |back: usize| prev(back).and_then(|n| n.utf8_text(src).ok()).unwrap_or("");
            match prev_text(1) {
                "." => {}
                "::" => out.push((*k, prev_text(2).to_string())),
                _ => out.push((*k, String::new())),
            }
        }
    }
    out
}

fn range(n: &Node) -> [i32; 4] {
    let (a, b) = (n.start_position(), n.end_position());
    [a.row as i32, a.column as i32, b.row as i32, b.column as i32]
}

/// Parse every supported file under `root` and resolve its calls. `project` names the project's own package in symbols.
pub fn index(root: &Path, project: &str) -> anyhow::Result<(Vec<Document>, usize)> {
    let (docs, resolved, rules) = parse(root, project)?;
    // Measurement hook: FOOTNOTE_CODE_RULES=<file> writes every link with its rule (file, line, name, target file, rule).
    if let Some(out) = std::env::var_os("FOOTNOTE_CODE_RULES") {
        let _ = std::fs::write(out, rules.join("\n"));
    }
    Ok((docs, resolved))
}

fn parse(root: &Path, project: &str) -> anyhow::Result<(Vec<Document>, usize, Vec<String>)> {
    let guesses = std::env::var_os("FOOTNOTE_CODE_GUESSES").is_some_and(|v| v == "1");
    let mut files: Vec<(String, Lang, Vec<u8>)> = Vec::new();
    let all = candidates(root);
    // Rust workspace crates by the name code uses (`footnote_core`) -> their folder (`crates/core/`).
    let crates: HashMap<String, String> = all
        .iter()
        .filter(|p| p.ends_with("Cargo.toml"))
        .filter_map(|p| {
            let text = std::fs::read_to_string(root.join(p)).ok()?;
            let name = text.split("[package]").nth(1)?.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "name").then(|| v.trim().trim_matches('"').replace('-', "_"))
            })?;
            Some((name, p.trim_end_matches("Cargo.toml").to_string()))
        })
        .collect();
    for rel in all {
        if files.len() >= MAX_FILES {
            break;
        }
        let Some(l) = lang_of(&rel) else { continue };
        let path = root.join(&rel);
        if std::fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() > MAX_FILE) {
            continue;
        }
        if let Ok(b) = std::fs::read(&path) {
            files.push((rel, l, b));
        }
    }
    let mut defs: Vec<Def> = Vec::new();
    let mut calls: Vec<Call> = Vec::new();
    let mut imports: Vec<HashMap<String, String>> = vec![HashMap::new(); files.len()]; // file -> bound name -> module
    let mut top_imports: Vec<HashSet<String>> = vec![HashSet::new(); files.len()];
    let mut compiled: HashMap<Lang, (Language, Query, Query)> = HashMap::new();
    // Rust struct field types, (struct, field) -> types: one step of a receiver chain (`self.store.save()`).
    let mut fields: HashMap<(String, String), Vec<(String, usize)>> = HashMap::new();
    // Java: the types a class or interface extends or implements, by its name.
    let mut supers: HashMap<String, Vec<String>> = HashMap::new();
    let rust_fields = Query::new(&tree_sitter_rust::LANGUAGE.into(), RUST_FIELDS).ok();
    let mut parser = Parser::new();
    for (i, (_, l, src)) in files.iter().enumerate() {
        if !compiled.contains_key(l) {
            let (lang, tags, imp) = grammar(*l);
            // The grammar's tags query plus calls it leaves out: Rust path calls (`a::f()`), measured as the biggest gap.
            // TypeScript's own query holds only TS-only items; functions, classes and calls come from the JavaScript one (#216).
            let tags = match l {
                Lang::Rust => format!("{tags}\n{RUST_PATH_CALLS}"),
                Lang::Ts | Lang::Tsx => format!("{}\n{tags}", tree_sitter_javascript::TAGS_QUERY),
                _ => tags.to_string(),
            };
            let (Ok(t), Ok(q)) = (Query::new(&lang, &tags), Query::new(&lang, imp)) else {
                continue;
            };
            compiled.insert(*l, (lang, t, q));
        }
        let (lang, tags, imp) = &compiled[l];
        if parser.set_language(lang).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(src, None) else {
            continue;
        };
        let names = tags.capture_names();
        let mut seen = HashSet::new();
        let mut cur = QueryCursor::new();
        let mut it = cur.matches(tags, tree.root_node(), src.as_slice());
        while let Some(m) = it.next() {
            let mut name = None;
            let mut role = None;
            for c in m.captures() {
                let cap = names[c.index as usize];
                if cap == "name" {
                    name = Some(c.node);
                } else if cap.starts_with("definition.") || cap.starts_with("reference.call") {
                    role = Some((cap, c.node));
                }
            }
            let (Some(n), Some((cap, node))) = (name, role) else {
                continue;
            };
            let Ok(text) = n.utf8_text(src) else { continue };
            if cap.starts_with("definition.") {
                if !seen.insert(node.id()) {
                    continue; // Rust methods match both `definition.method` and `definition.function`
                }
                let kind = match cap {
                    "definition.function" | "definition.method" => "function",
                    // `mod store;`: a module is a path step, never the owner of `Type::f()`.
                    "definition.module" => "module",
                    _ => "type",
                };
                let signature = node
                    .utf8_text(src)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim();
                let (owner, ret) = if kind == "function" && *l != Lang::Go {
                    // The `impl`, trait or class whose body holds this function (a Python decorator wraps it once).
                    let owner = node
                        .parent()
                        .and_then(|p| match p.kind() {
                            "decorated_definition" => p.parent(),
                            _ => Some(p),
                        })
                        .filter(|p| {
                            matches!(
                                p.kind(),
                                "declaration_list"
                                    | "block"
                                    | "class_body"
                                    | "interface_body"
                                    | "object_type"
                            )
                        })
                        .and_then(|p| p.parent())
                        .and_then(|p| owner_name(&p, src))
                        // `{ getPath: () => .. }` and `Deno.upgradeWebSocket = () => ..` bind no name: a member of no
                        // known type, so a bare call never reaches them (hono, 18 Nov 2026).
                        .or_else(|| {
                            let member = node.kind() == "pair"
                                || node
                                    .child_by_field_name("left")
                                    .is_some_and(|x| x.kind() == "member_expression");
                            member.then(String::new)
                        });
                    let ret = node
                        .child_by_field_name(if *l == Lang::Java {
                            "type"
                        } else {
                            "return_type"
                        })
                        .and_then(|r| r.utf8_text(src).ok())
                        .map(|r| r.trim_start_matches(':').trim())
                        .map(|r| owner.as_deref().map_or(r.to_string(), |o| swap_self(r, o)));
                    (owner, ret)
                } else {
                    (None, None)
                };
                defs.push(Def {
                    name: text.into(),
                    kind,
                    file: i,
                    range: range(&n),
                    span: range(&node),
                    signature: signature.chars().take(200).collect(),
                    owner,
                    ret,
                    arity: java_arity(node, *l),
                });
            } else if node.kind() != "macro_invocation" {
                // The JavaScript query tags `obj.f(x)` on its arguments; the call is their parent.
                let node = if matches!(node.kind(), "arguments" | "argument_list") {
                    node.parent().unwrap_or(node)
                } else {
                    node
                };
                // A Rust macro (`env!`, `write!`) is not a call to a function of that name.
                let (qualifier, member) = qualifier(&node, &n, src);
                let f = node.child_by_field_name("function");
                let recv = if member || (*l == Lang::Java && qualifier.is_empty()) {
                    match l {
                        Lang::Rust => f
                            .and_then(|f| f.child_by_field_name("value"))
                            .and_then(|v| receiver(v, src, 0)),
                        Lang::Python | Lang::Ts | Lang::Tsx | Lang::Js => f
                            .and_then(|f| f.child_by_field_name("object"))
                            .and_then(|v| receiver_dyn(v, src, 0)),
                        Lang::Java => node
                            .child_by_field_name("object")
                            .and_then(|v| receiver_java(v, src, 0))
                            // A bare call is a call on `this`.
                            .or_else(|| {
                                (!member && !in_anonymous(node))
                                    .then(|| (Base::Type("Self".into()), vec![]))
                            }),
                        _ => None,
                    }
                    .and_then(|(b, s)| {
                        // `Self` and `self` mean the impl the call sits in.
                        let o = impl_owner(&node, src);
                        let fix = |t: String| match &o {
                            Some(o) => Some(swap_self(&t, o)),
                            None => (!has_word(&t, "Self")).then_some(t),
                        };
                        Some((
                            match b {
                                Base::Type(t) => Base::Type(fix(t)?),
                                Base::Static(t, f) => Base::Static(fix(t)?, f),
                            },
                            s,
                        ))
                    })
                } else {
                    None
                };
                let qualifier_is_empty = qualifier.is_empty();
                calls.push(Call {
                    name: text.into(),
                    qualifier,
                    member,
                    recv,
                    args: (*l == Lang::Java)
                        .then(|| {
                            node.child_by_field_name("arguments")
                                .map(|a| a.named_child_count())
                        })
                        .flatten(),
                    within: if *l == Lang::Java && !member && qualifier_is_empty {
                        enclosing_classes(node, src)
                    } else {
                        Vec::new()
                    },
                    file: i,
                    range: range(&n),
                    in_macro: false,
                });
            } else if *l == Lang::Rust {
                // A macro's arguments are an unparsed token tree: `f(..)` and `a::f(..)` inside them are calls (#197).
                for (name, qualifier) in macro_calls(node, src) {
                    calls.push(Call {
                        name: name.utf8_text(src).unwrap_or_default().into(),
                        qualifier,
                        member: false,
                        recv: None,
                        args: None,
                        within: Vec::new(),
                        file: i,
                        range: range(&name),
                        in_macro: true,
                    });
                }
            }
        }
        if let (Lang::Rust, Some(q)) = (l, &rust_fields) {
            let mut cur = QueryCursor::new();
            let mut it = cur.matches(q, tree.root_node(), src.as_slice());
            while let Some(m) = it.next() {
                let t: Vec<&str> = m
                    .captures()
                    .iter()
                    .map(|c| c.node.utf8_text(src).unwrap_or(""))
                    .collect();
                if let [s, f, ty] = t[..] {
                    fields
                        .entry((s.into(), f.into()))
                        .or_default()
                        .push((swap_self(ty, s), i));
                }
            }
        }
        if matches!(
            l,
            Lang::Python | Lang::Ts | Lang::Tsx | Lang::Js | Lang::Java
        ) {
            let found = if *l == Lang::Java {
                java_fields(tree.root_node(), src)
            } else {
                class_fields(tree.root_node(), src)
            };
            let sup = if *l == Lang::Java {
                java_supers(tree.root_node(), src)
            } else {
                class_supers(tree.root_node(), src)
            };
            for (class, s) in sup {
                supers.entry(class).or_default().push(s);
            }
            for (s, f, t) in found {
                let e = fields.entry((s, f)).or_default();
                if !e.contains(&(t.clone(), i)) {
                    e.push((t, i));
                }
            }
        }
        let mut cur = QueryCursor::new();
        let mut it = cur.matches(imp, tree.root_node(), src.as_slice());
        while let Some(m) = it.next() {
            for c in m.captures() {
                // At the top of the file: the import (or Rust's `use` around its argument) sits right under the root.
                let mut p = c.node.parent();
                while let Some(x) = p.filter(|x| x.kind() == "use_declaration") {
                    p = x.parent();
                }
                let top = p.is_some_and(|x| x.parent().is_none());
                if let Ok(text) = c.node.utf8_text(src) {
                    for (bound, module) in bindings(*l, text) {
                        if top {
                            top_imports[i].insert(bound.clone());
                        }
                        imports[i].insert(bound, module);
                    }
                }
            }
        }
    }

    // Which modules are the project's own: relative paths, `crate`/`self`/`super`, or a name some project path carries.
    let stems: HashSet<String> = files
        .iter()
        .flat_map(|(p, ..)| {
            p.split('/')
                .map(|s| s.split('.').next().unwrap_or(s).replace('-', "_"))
                .collect::<Vec<_>>()
        })
        .collect();
    // Java packages the project holds, as every tail of a source folder: `org/acme/store` for `src/main/java/org/acme/store`.
    // Its folders are named `java` and `main` too, so a stem match would call `java.util.Set` local.
    let java_dirs: HashSet<String> = files
        .iter()
        .filter(|f| f.1 == Lang::Java)
        .filter_map(|f| f.0.rsplit_once('/').map(|d| d.0))
        .flat_map(|d| {
            d.char_indices()
                .filter(|&(_, c)| c == '/')
                .map(|(i, _)| &d[i + 1..])
                .chain([d])
        })
        .map(str::to_string)
        .collect();
    let local = |m: &str, file: usize| {
        if files[file].1 == Lang::Java {
            // `a.b.C` or `a.b.C.f` (a static import): the package is `a/b` or `a/b/C`'s parent.
            let path = m.replace('.', "/");
            return std::iter::successors(Some(path.as_str()), |p| p.rsplit_once('/').map(|x| x.0))
                .skip(1)
                .take(2)
                .any(|d| java_dirs.contains(d));
        }
        m.starts_with('.')
            || m.starts_with('/')
            || ["crate", "self", "super"].contains(&first(m))
            || stems.contains(&first(m).replace('-', "_"))
            || crates.contains_key(first(m)) // a workspace crate (`use deep::helper`)
    };
    let by_name: HashMap<&str, Vec<usize>> =
        defs.iter()
            .enumerate()
            .fold(HashMap::new(), |mut m, (i, d)| {
                m.entry(d.name.as_str()).or_default().push(i);
                m
            });
    // Method tables: (type, method) -> its definitions, from every `impl Type` and `trait Type` block.
    let methods: HashMap<(&str, &str), Vec<usize>> =
        defs.iter()
            .enumerate()
            .fold(HashMap::new(), |mut m, (i, d)| {
                if let Some(o) = &d.owner {
                    m.entry((o.as_str(), d.name.as_str())).or_default().push(i);
                }
                m
            });
    let owner = |ty: &str, file: usize| {
        let rust = files[file].1 == Lang::Rust;
        // Qualified types need module resolution; stripping their path could bind a dependency to our own type.
        let owner = if rust {
            if ty.contains("::") {
                return None;
            }
            head(ty)
        } else {
            head_dyn(ty)?
        };
        // Imported type identity is not established by this table, except a TypeScript or Python class imported from
        // the project's own module: its name is checked unique below.
        if imports[file]
            .get(&owner)
            .is_some_and(|m| rust || !local(m, file))
        {
            return None;
        }
        // Equal short names in different modules are distinct types. Never combine their method tables.
        let types = by_name.get(owner.as_str())?;
        if types.iter().filter(|&&d| defs[d].kind == "type").count() != 1 {
            return None;
        }
        Some(owner)
    };
    // The file that declares a type, when the name is unique.
    let home = |o: &str| -> Option<usize> {
        match by_name
            .get(o)?
            .iter()
            .filter(|&&d| defs[d].kind == "type")
            .collect::<Vec<_>>()[..]
        {
            [d] => Some(defs[*d].file),
            _ => None,
        }
    };
    // The method of that type, or else of the nearest ancestors that define the name: one of them, or none.
    let method = |ty: &str, name: &str, file: usize, args: Option<usize>| {
        let mut level = vec![owner(ty, file)?];
        let mut seen: HashSet<String> = level.iter().cloned().collect();
        for _ in 0..8 {
            let defined: Vec<&Vec<usize>> = level
                .iter()
                .filter_map(|o| methods.get(&(o.as_str(), name)))
                .collect();
            if !defined.is_empty() {
                let found: Vec<usize> = defined
                    .iter()
                    .flat_map(|v| narrow(&defs, v, args))
                    .collect();
                return match found.as_slice() {
                    [one] => Some(*one),
                    _ => None,
                };
            }
            // A field of that name (`match = match` in a class) hides the base's method: unknown, not the base's.
            if level
                .iter()
                .any(|o| fields.contains_key(&(o.clone(), name.to_string())))
            {
                return None;
            }
            level = level
                .iter()
                .flat_map(|o| {
                    let at = home(o);
                    supers
                        .get(o)
                        .into_iter()
                        .flatten()
                        .filter_map(move |s| owner(s, at?))
                })
                .filter(|o| seen.insert(o.clone()))
                .collect();
            if level.is_empty() {
                return None;
            }
        }
        None
    };
    // The type a receiver chain ends on; `None` as soon as one step is unknown (std, a dependency, a generic).
    let type_of = |b: &Base, steps: &[Step], file: usize| -> Option<(String, usize)> {
        let (mut t, mut context) = match b {
            Base::Type(t) => (t.clone(), file),
            Base::Static(ty, f) => {
                let d = &defs[method(ty, f, file, None)?];
                (d.ret.clone()?, d.file)
            }
        };
        for s in steps {
            t = match s {
                Step::Try => unwrap_ty(&t)?,
                Step::Call(m) if m == "unwrap" || m == "expect" => unwrap_ty(&t)?,
                Step::Call(m) => {
                    let d = &defs[method(&t, m, context, None)?];
                    context = d.file;
                    d.ret.clone()?
                }
                Step::Field(f) => match fields.get(&(owner(&t, context)?, f.clone()))?.as_slice() {
                    [(one, file)] => {
                        context = *file;
                        one.clone()
                    }
                    _ => return None,
                },
            };
        }
        Some((t, context))
    };
    let in_module = |cands: &[usize], module: &str| -> Vec<usize> {
        let last = module
            .trim_start_matches(['.', '/'])
            .rsplit(['/', '.', ':'])
            .find(|s| !s.is_empty())
            .unwrap_or("");
        cands
            .iter()
            .copied()
            .filter(|&d| {
                files[defs[d].file]
                    .0
                    .split('/')
                    .enumerate()
                    // A folder by its name, the file by its exact stem: `./hono` is not `hono.test.ts` (hono).
                    .any(|(i, seg)| {
                        let file = i + 1 == files[defs[d].file].0.split('/').count();
                        Some(last)
                            == if file {
                                seg.rsplit_once('.').map(|s| s.0)
                            } else {
                                Some(seg)
                            }
                    })
            })
            .collect()
    };

    let sym = |d: &Def| {
        format!(
            "treesitter local {project} . {}/{}:{}{}",
            files[d.file].0,
            d.name,
            d.range[0],
            match d.kind {
                "function" => "().",
                "module" => "/",
                _ => "#",
            }
        )
    };
    let mut docs: Vec<Document> = files
        .iter()
        .map(|(p, ..)| Document {
            path: p.clone(),
            occurrences: vec![],
            symbols: vec![],
            unresolved: vec![],
        })
        .collect();
    for d in &defs {
        let s = sym(d);
        docs[d.file].symbols.push(Symbol {
            symbol: s.clone(),
            display_name: d.name.clone(),
            signature: d.signature.clone(),
        });
        docs[d.file].occurrences.push(Occurrence {
            range: d.range,
            symbol: s,
            roles: DEFINITION,
            enclosing: Some(d.span),
            ..Default::default()
        });
    }
    let mut resolved = 0;
    let mut rules: Vec<String> = Vec::new();
    // For `binds_locally`: each file decoded once, its line offsets and its own definitions.
    let texts: Vec<String> = files
        .iter()
        .map(|f| String::from_utf8_lossy(&f.2).into_owned())
        .collect();
    let starts: Vec<Vec<usize>> = texts
        .iter()
        .map(|t| {
            std::iter::once(0)
                .chain(t.match_indices('\n').map(|(i, _)| i + 1))
                .collect()
        })
        .collect();
    let mut mine: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    for (i, d) in defs.iter().enumerate() {
        mine[d.file].push(i);
    }
    for c in &calls {
        let cands = by_name
            .get(c.name.as_str())
            .map_or(&[][..], |v| v.as_slice());
        let same: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&d| defs[d].file == c.file)
            // A bare Rust call (`write_help(..)`) never reaches a method; clap's `Command::write_help` caught this.
            .filter(|&d| {
                !(c.qualifier.is_empty() && defs[d].owner.is_some())
                    // A bare Java call is on `this` or on a class around it, never on another class of the file.
                    || defs[d].owner.as_ref().is_some_and(|o| c.within.contains(o))
            })
            .collect();
        let same = narrow(&defs, &same, c.args);
        let bare = c.qualifier.is_empty() && !c.member;
        let via = if c.member && c.qualifier.is_empty() {
            None // called on a computed value (`new Hono().get()`): never through an import (hono, 18 Nov 2026)
        } else if c.qualifier.is_empty() {
            imports[c.file].get(&c.name)
        } else {
            imports[c.file].get(&c.qualifier)
        };
        // A name the calling item binds itself (a closure in a `let`, a parameter) is not a project function.
        if bare && binds_locally(&texts[c.file], &starts[c.file], &defs, &mine[c.file], c) {
            continue;
        }
        let rust = files[c.file].1 == Lang::Rust;
        // `Type::f()` or `Class.f()`: the `f` defined in the file that defines `Type`, when that is one definition.
        let owned_by = |q: &str| -> Option<usize> {
            if q.is_empty() {
                return None;
            }
            let homes: HashSet<usize> = by_name
                .get(q)
                .into_iter()
                .flatten()
                .filter(|&&d| defs[d].kind == "type")
                .map(|&d| defs[d].file)
                .collect();
            let in_homes: Vec<usize> = cands
                .iter()
                .copied()
                .filter(|d| homes.contains(&defs[*d].file))
                .collect();
            match narrow(&defs, &in_homes, c.args).as_slice() {
                [one] => Some(*one),
                _ => None,
            }
        };
        // Each link names the rule that made it, so its precision can be measured per rule (FOOTNOTE_CODE_RULES below).
        let typed = c
            .recv
            .as_ref()
            .and_then(|(b, s)| type_of(b, s, c.file))
            .and_then(|(t, file)| method(&t, &c.name, file, c.args));
        let target: Option<(usize, &str)> = if let Some(d) = typed {
            Some((d, "receiver_type"))
        } else if !same.is_empty()
            // A name imported at the top of the file wins over a same-file definition of it (hono's
            // `Deno.upgradeWebSocket = ...` in a test). Only the top: `use super::f` inside `mod tests` must not hide `f`.
            && ((bare && !top_imports[c.file].contains(&c.name))
                || ["self", "this", "Self"].contains(&c.qualifier.as_str()))
        {
            Some((same[0], "same_file"))
        } else if let Some(module) = via {
            if !local(module, c.file) {
                // A call into an outside package: counted for the architecture map, never linked.
                let pkg = package_of(files[c.file].1, module);
                docs[c.file].occurrences.push(Occurrence {
                    range: c.range,
                    symbol: format!("treesitter pkg {pkg} . {}.", c.name),
                    roles: 0,
                    enclosing: None,
                    ..Default::default()
                });
                continue;
            }
            // From a workspace crate: its definitions, narrowed by the module path inside it when one is given.
            let found = match crates.get(first(module)) {
                Some(home) => {
                    let inside: Vec<usize> = cands
                        .iter()
                        .copied()
                        .filter(|&x| files[defs[x].file].0.starts_with(home.as_str()))
                        .collect();
                    if module.contains("::") && in_module(&inside, module).len() == 1 {
                        in_module(&inside, module)
                    } else {
                        inside
                    }
                }
                // A relative JavaScript or TypeScript import names one file: `./websocket` beside the importer, as a file
                // or a folder's index; `adapter/bun/` must not reach `helper/websocket/` (hono, 18 Nov 2026).
                None if module.starts_with('.')
                    && matches!(files[c.file].1, Lang::Ts | Lang::Tsx | Lang::Js) =>
                {
                    let base = relative(&files[c.file].0, module);
                    cands
                        .iter()
                        .copied()
                        .filter(|&x| {
                            let p = &files[defs[x].file].0;
                            let stem = p.rsplit_once('.').map_or(p.as_str(), |s| s.0);
                            base.as_deref()
                                .is_some_and(|b| stem == b || stem == format!("{b}/index"))
                        })
                        .collect()
                }
                None => in_module(cands, module),
            };
            match narrow(&defs, &found, c.args).as_slice() {
                [one] => Some((*one, "import")),
                _ => None,
            }
        } else if c.member {
            None // a method on some value: its type is unknown here, and std or a dependency may define it
        } else if let Some(one) = if rust {
            // Rust: only a method of that very type. The file rule linked `ByteBuf::default()` (derived) to a neighbour's
            // `impl Default for Phase` (brotli, 18 Nov 2026).
            method(&c.qualifier, &c.name, c.file, c.args)
        } else {
            owned_by(&c.qualifier)
        } {
            Some((one, "type_path"))
        } else if rust && owner(&c.qualifier, c.file).is_some() {
            None // a project type without that method: derived, a trait default, or a trait from a dependency
        } else if !c.qualifier.is_empty() {
            // `footnote_core::hash()`, `store::open()`: the qualifier names a crate (its folder) or a module (its file).
            let home = crates.get(c.qualifier.as_str());
            let found: Vec<usize> = match home {
                Some(d) => cands
                    .iter()
                    .copied()
                    .filter(|&x| files[defs[x].file].0.starts_with(d.as_str()))
                    .collect(),
                None => in_module(cands, &c.qualifier),
            };
            match found.as_slice() {
                [one] => Some((*one, "module_path")),
                _ => None,
            }
        } else {
            // A bare name defined in another file, not imported: measured 0 right in 12 (17 Nov 2026), so never linked.
            None
        };
        if let Some((t, rule)) = target {
            docs[c.file].occurrences.push(Occurrence {
                range: c.range,
                symbol: sym(&defs[t]),
                roles: if c.in_macro { MACRO_ARG } else { 0 },
                enclosing: None,
                ..Default::default()
            });
            rules.push(format!(
                "{}\t{}\t{}\t{}\t{rule}",
                files[c.file].0,
                c.range[0] + 1,
                c.name,
                files[defs[t].file].0
            ));
            resolved += 1;
        } else {
            // Lower tiers (opt-in, FOOTNOTE_CODE_GUESSES=1): kept apart from the links above, so the default graph is unchanged.
            let by_receiver_name = || {
                // `store.save()`: a receiver named like a project type (`Store`, `my_store` -> `MyStore`), when that type has the method.
                let ty: String = c
                    .qualifier
                    .split('_')
                    .map(|w| {
                        let mut ch = w.chars();
                        ch.next()
                            .map(|f| f.to_uppercase().chain(ch).collect::<String>())
                            .unwrap_or_default()
                    })
                    .collect();
                (c.member && !ty.is_empty() && c.qualifier.chars().all(is_ident))
                    .then(|| method(&ty, &c.name, c.file, c.args))
                    .flatten()
            };
            // Tried and dropped (19 Nov 2026): a name defined once in the project (member 0.04 to 0.69, bare 0.0 to 0.65 precision).
            let guess: Option<(usize, &'static str)> = if guesses {
                by_receiver_name().map(|d| (d, "guess_receiver_name"))
            } else {
                None
            };
            if let Some((t, rule)) = guess {
                docs[c.file].occurrences.push(Occurrence {
                    range: c.range,
                    symbol: sym(&defs[t]),
                    roles: GUESS,
                    enclosing: None,
                    rule,
                });
                rules.push(format!(
                    "{}	{}	{}	{}	{rule}",
                    files[c.file].0,
                    c.range[0] + 1,
                    c.name,
                    files[defs[t].file].0
                ));
            }
            // The report: where the call is and which project definitions share its name, so an agent knows where to grep.
            docs[c.file].unresolved.push(Unresolved {
                line: c.range[0] + 1,
                name: c.name.clone(),
                candidates: cands
                    .iter()
                    .take(5)
                    .map(|&d| format!("{}:{}", files[defs[d].file].0, defs[d].range[0] + 1))
                    .collect(),
            });
        }
    }
    Ok((docs, resolved, rules))
}

/// Java overloads: of several definitions of one name, those taking as many parameters as the call has arguments. Abstains
/// (keeps them all) when the count is unknown or one of them is varargs, and leaves a single definition alone.
fn narrow(defs: &[Def], v: &[usize], args: Option<usize>) -> Vec<usize> {
    match args {
        Some(n) if v.len() > 1 && v.iter().all(|&d| defs[d].arity.is_some()) => v
            .iter()
            .copied()
            .filter(|&d| defs[d].arity == Some(n))
            .collect(),
        _ => v.to_vec(),
    }
}

/// A Java method's parameter count (`None` when it is varargs or not Java).
fn java_arity(node: Node, l: Lang) -> Option<usize> {
    if l != Lang::Java {
        return None;
    }
    let params = node.child_by_field_name("parameters")?;
    let mut c = params.walk();
    let kinds: Vec<&str> = params.named_children(&mut c).map(|p| p.kind()).collect();
    (!kinds.contains(&"spread_parameter"))
        .then(|| kinds.iter().filter(|k| **k == "formal_parameter").count())
}

/// Whether the innermost item around a call declares the called name before the call: `let f =`, `const f`, `var f`,
/// `f := `, `f = `, a parameter `f:` or a closure argument `|f|`. Text patterns, not scopes: a miss only loses a link.
/// `text` is the file decoded once, `starts` its line offsets and `mine` its definitions: per call, a scan of every
/// definition in the project and a fresh decode of the file made large repositories quadratic (#246).
fn binds_locally(text: &str, starts: &[usize], defs: &[Def], mine: &[usize], c: &Call) -> bool {
    let Some(item) = mine
        .iter()
        .map(|&d| &defs[d])
        .filter(|d| d.span[0] <= c.range[0] && c.range[0] <= d.span[2])
        .min_by_key(|d| d.span[2] - d.span[0])
    else {
        return false;
    };
    let at = |line: i32| starts.get(line as usize).copied().unwrap_or(text.len());
    let before = &text[at(item.span[0])..at(c.range[0] + 1)];
    let n = &c.name;
    [
        format!("let {n} "),
        format!("let {n}="),
        format!("let mut {n} "),
        format!("const {n} "),
        format!("const {n}="),
        format!("var {n} "),
        format!("var {n}="),
        format!("{n} := "),
        format!(" {n} = "),
        format!("({n}:"),
        format!(", {n}:"),
        format!("|{n}|"),
        format!("|{n},"),
        format!("({n},"),
        format!("({n})"),
    ]
    .iter()
    .any(|p| before.contains(p.as_str()))
}

/// The type an `impl` block is for (`impl<T> Trait for Store<T>` -> `Store`), or a trait's name.
fn owner_name(item: &Node, src: &[u8]) -> Option<String> {
    let field = match item.kind() {
        "impl_item" => "type",
        "trait_item"
        | "class_definition"
        | "class_declaration"
        | "abstract_class_declaration"
        | "class"
        | "record_declaration"
        | "enum_declaration"
        | "interface_declaration" => "name",
        _ => return None,
    };
    Some(head(item.child_by_field_name(field)?.utf8_text(src).ok()?))
}

fn impl_owner(n: &Node, src: &[u8]) -> Option<String> {
    let mut p = n.parent();
    while let Some(x) = p {
        if let Some(o) = owner_name(&x, src) {
            return Some(o);
        }
        p = x.parent();
    }
    None
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte offsets where `w` occurs as a whole word in `t`.
fn word_at(t: &str, w: &str) -> Vec<usize> {
    t.match_indices(w)
        .filter(|(i, _)| {
            !t[..*i].chars().next_back().is_some_and(is_ident)
                && !t[i + w.len()..].chars().next().is_some_and(is_ident)
        })
        .map(|(i, _)| i)
        .collect()
}

fn has_word(t: &str, w: &str) -> bool {
    !word_at(t, w).is_empty()
}

fn swap_self(t: &str, owner: &str) -> String {
    let mut out = t.to_string();
    for i in word_at(t, "Self").into_iter().rev() {
        out.replace_range(i..i + 4, owner);
    }
    out
}

/// The type's own name: `&'a mut crate::x::Store<T>` -> `Store`; `Box`, `Arc` and `Rc` are looked through.
fn head(t: &str) -> String {
    let mut t = t.trim();
    loop {
        let mut s = t.trim_start_matches('&').trim_start();
        if s.starts_with('\'') {
            s = s.split_once(' ').map_or("", |x| x.1);
        }
        for p in ["mut ", "dyn ", "impl "] {
            s = s.trim_start_matches(p).trim_start();
        }
        let name = s
            .split('<')
            .next()
            .unwrap_or("")
            .rsplit("::")
            .next()
            .unwrap_or("")
            .trim();
        match (["Box", "Arc", "Rc"].contains(&name), generic_arg(s)) {
            (true, Some(inner)) => t = inner,
            _ => return name.to_string(),
        }
    }
}

/// The first type argument: `Result<Store, E>` -> `Store`.
fn generic_arg(t: &str) -> Option<&str> {
    let inner = &t[t.find('<')? + 1..t.rfind('>')?];
    let mut depth = 0;
    for (i, ch) in inner.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => return Some(inner[..i].trim()),
            _ => {}
        }
    }
    Some(inner.trim())
}

/// What `?`, `unwrap()` or `expect()` leaves of a `Result` or an `Option`.
fn unwrap_ty(t: &str) -> Option<String> {
    let s = t.trim().trim_start_matches('&').trim_start();
    let name = s.split('<').next()?.rsplit("::").next()?.trim();
    // `Promise`: what TypeScript's `await` leaves.
    ["Result", "Option", "Promise"]
        .contains(&name)
        .then(|| generic_arg(s).map(str::to_string))
        .flatten()
}

/// Trace a Rust expression to where its type is written: `self`, a typed `let` or parameter, `Type { .. }`, `Type::f()`,
/// followed by fields, method calls and `?`. Anything else (a closure argument, a pattern, a macro) gives up.
fn receiver(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    if depth > 12 {
        return None;
    }
    let text = |n: Node| n.utf8_text(src).ok().map(str::to_string);
    let then = |inner: Node, step: Step| {
        let (b, mut s) = receiver(inner, src, depth + 1)?;
        s.push(step);
        Some((b, s))
    };
    match e.kind() {
        "self" => Some((Base::Type("Self".into()), vec![])),
        "identifier" => declared(e, src, depth),
        "field_expression" => then(
            e.child_by_field_name("value")?,
            Step::Field(text(e.child_by_field_name("field")?)?),
        ),
        "call_expression" => {
            let f = e.child_by_field_name("function")?;
            match f.kind() {
                "field_expression" => then(
                    f.child_by_field_name("value")?,
                    Step::Call(text(f.child_by_field_name("field")?)?),
                ),
                "scoped_identifier" => Some((
                    Base::Static(
                        text(f.child_by_field_name("path")?)?,
                        text(f.child_by_field_name("name")?)?,
                    ),
                    vec![],
                )),
                _ => None,
            }
        }
        "try_expression" => then(e.named_child(0)?, Step::Try),
        "reference_expression" => receiver(e.child_by_field_name("value")?, src, depth + 1),
        "parenthesized_expression" | "await_expression" => {
            receiver(e.named_child(0)?, src, depth + 1)
        }
        "struct_expression" => Some((Base::Type(text(e.child_by_field_name("name")?)?), vec![])),
        _ => None,
    }
}

/// Where a Rust variable is declared before `e`: the nearest `let` in an enclosing block (so shadowing holds) or a
/// parameter of the enclosing function. A closure argument, a loop, `if let` or match pattern with that name gives up.
fn declared(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    let name = e.utf8_text(src).ok()?;
    let mut child = e;
    while let Some(p) = child.parent() {
        match p.kind() {
            "block" => {
                let mut c = p.walk();
                let lets: Vec<Node> = p
                    .named_children(&mut c)
                    .take_while(|k| k.end_byte() <= child.start_byte())
                    .filter(|k| k.kind() == "let_declaration")
                    .collect();
                for l in lets.iter().rev() {
                    let pat = l.child_by_field_name("pattern")?.utf8_text(src).ok()?;
                    if pat.trim_start_matches("mut ").trim() == name {
                        if let Some(t) = l.child_by_field_name("type") {
                            return Some((Base::Type(t.utf8_text(src).ok()?.into()), vec![]));
                        }
                        return receiver(l.child_by_field_name("value")?, src, depth + 1);
                    }
                    if has_word(pat, name) {
                        return None; // bound inside a tuple or struct pattern
                    }
                }
            }
            "function_item" => {
                let params = p.child_by_field_name("parameters")?;
                let mut c = params.walk();
                let found = params
                    .named_children(&mut c)
                    .filter(|k| k.kind() == "parameter")
                    .find(|k| {
                        k.child_by_field_name("pattern")
                            .and_then(|x| x.utf8_text(src).ok())
                            .is_some_and(|x| x.trim_start_matches("mut ").trim() == name)
                    })?;
                return Some((
                    Base::Type(
                        found
                            .child_by_field_name("type")?
                            .utf8_text(src)
                            .ok()?
                            .into(),
                    ),
                    vec![],
                ));
            }
            "closure_expression" | "for_expression" | "if_expression" | "while_expression"
            | "match_arm" => {
                let head = String::from_utf8_lossy(&src[p.start_byte()..child.start_byte()]);
                if has_word(&head, name) {
                    return None;
                }
            }
            _ => {}
        }
        child = p;
    }
    None
}

/// A TypeScript or Python type's own class: `Store | null`, `Optional[Store]`, `"Store"` -> `Store`. Arrays, unions of
/// classes and qualified names (`models.Store`) give up; `list[Store]` comes out as `list`, which no project class owns.
fn head_dyn(t: &str) -> Option<String> {
    let t = t
        .trim()
        .trim_start_matches(':')
        .trim()
        .trim_matches(['"', '\'']);
    let t = t
        .strip_prefix("Optional[")
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(t);
    let parts: Vec<&str> = t
        .split('|')
        .map(str::trim)
        .filter(|p| !["None", "null", "undefined"].contains(p))
        .collect();
    let [one] = parts[..] else { return None };
    let name = one.split(['<', '[']).next()?.trim();
    (!one.contains("[]") && !name.is_empty() && name.chars().all(is_ident))
        .then(|| name.to_string())
}

/// The class a TypeScript or Python value builds: `new Store()` or `Store()`. `None`/`null` add nothing; any other value is
/// `?`, so a field assigned it stays ambiguous.
fn value_type(v: Node, src: &[u8]) -> Option<String> {
    let callee = match v.kind() {
        "call" => v.child_by_field_name("function"),
        "new_expression" => v.child_by_field_name("constructor"),
        "none" | "null" | "undefined" => return None,
        _ => None,
    };
    Some(
        callee
            .filter(|c| c.kind() == "identifier")
            .and_then(|c| c.utf8_text(src).ok())
            .unwrap_or("?")
            .to_string(),
    )
}

/// TypeScript and Python field types as (class, field, type): annotated fields, `self.x = T()` and `this.x = new T()` in any
/// method, class-level `x = T()`, and TypeScript constructor parameter properties.
fn class_fields(root: Node, src: &[u8]) -> Vec<(String, String, String)> {
    let text = |n: Node| n.utf8_text(src).unwrap_or("").to_string();
    let mut out = Vec::new();
    // An explicit stack: a recursive walk could overflow on very deep expressions.
    let mut stack = vec![(root, String::new())];
    while let Some((n, outer)) = stack.pop() {
        let cls = owner_name(&n, src).unwrap_or(outer);
        let found = if cls.is_empty() {
            None
        } else {
            match n.kind() {
                "assignment" | "assignment_expression" => {
                    n.child_by_field_name("left").and_then(|l| {
                        let field = match l.kind() {
                            "attribute" | "member_expression" => {
                                let o = l.child_by_field_name("object")?;
                                if !matches!(text(o).as_str(), "self" | "this") {
                                    return None;
                                }
                                l.child_by_field_name("attribute")
                                    .or_else(|| l.child_by_field_name("property"))?
                            }
                            // Python class-level `x: T` or `x = T()`.
                            "identifier"
                                if n.parent()
                                    .and_then(|p| p.parent())
                                    .and_then(|b| b.parent())
                                    .is_some_and(|c| c.kind() == "class_definition") =>
                            {
                                l
                            }
                            _ => return None,
                        };
                        let ty = match n.child_by_field_name("type") {
                            Some(t) => text(t),
                            None => value_type(n.child_by_field_name("right")?, src)?,
                        };
                        Some((text(field), ty))
                    })
                }
                "public_field_definition" => n.child_by_field_name("name").and_then(|f| {
                    let ty = match n.child_by_field_name("type") {
                        Some(t) => text(t),
                        None => value_type(n.child_by_field_name("value")?, src)?,
                    };
                    Some((text(f), ty))
                }),
                // `constructor(private store: Store)`
                "required_parameter" | "optional_parameter" => {
                    let mut c = n.walk();
                    let property = n
                        .children(&mut c)
                        .any(|k| k.kind() == "accessibility_modifier" || k.kind() == "readonly");
                    property
                        .then(|| {
                            Some((
                                text(n.child_by_field_name("pattern")?),
                                text(n.child_by_field_name("type")?),
                            ))
                        })
                        .flatten()
                }
                _ => None,
            }
        };
        if let Some((f, t)) = found {
            out.push((cls.clone(), f, t.trim_start_matches(':').trim().to_string()));
        }
        let mut c = n.walk();
        for k in n.children(&mut c) {
            stack.push((k, cls.clone()));
        }
    }
    out
}

/// Trace a TypeScript, JavaScript or Python receiver to a written type: `this`/`self`, an annotated or constructed local,
/// an annotated parameter, `new T()` or `T()`, followed by fields, method calls and `await`. Anything else gives up.
fn receiver_dyn(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    if depth > 12 {
        return None;
    }
    let text = |n: Node| n.utf8_text(src).ok().map(str::to_string);
    let then = |inner: Node, step: Step| {
        let (b, mut s) = receiver_dyn(inner, src, depth + 1)?;
        s.push(step);
        Some((b, s))
    };
    let member = |m: Node| {
        let name = m
            .child_by_field_name("property")
            .or_else(|| m.child_by_field_name("attribute"))?;
        text(name)
    };
    match e.kind() {
        "this" => Some((Base::Type("Self".into()), vec![])),
        "identifier" if text(e)? == "self" => Some((Base::Type("Self".into()), vec![])),
        "identifier" => declared_dyn(e, src, depth),
        "member_expression" | "attribute" => {
            then(e.child_by_field_name("object")?, Step::Field(member(e)?))
        }
        "call_expression" | "call" => {
            let f = e.child_by_field_name("function")?;
            match f.kind() {
                "member_expression" | "attribute" => {
                    then(f.child_by_field_name("object")?, Step::Call(member(f)?))
                }
                // Python `Store(...)`: a constructor when `Store` is a project class (the method tables check it).
                "identifier" if e.kind() == "call" => Some((Base::Type(text(f)?), vec![])),
                _ => None,
            }
        }
        "new_expression" => {
            let c = e
                .child_by_field_name("constructor")
                .filter(|c| c.kind() == "identifier")?;
            Some((Base::Type(text(c)?), vec![]))
        }
        "await_expression" => then(e.named_child(0)?, Step::Try),
        "await" | "parenthesized_expression" | "non_null_expression" => {
            receiver_dyn(e.named_child(0)?, src, depth + 1)
        }
        _ => None,
    }
}

/// Where a TypeScript or Python variable gets its type before `e`: the nearest declaration or assignment in an enclosing
/// block, or an annotated parameter. A rebinding in between, a loop, `with`, `except` or comprehension variable gives up.
fn declared_dyn(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    let name = e.utf8_text(src).ok()?;
    let text = |n: Node| n.utf8_text(src).unwrap_or("");
    let mut child = e;
    while let Some(p) = child.parent() {
        match p.kind() {
            "block" | "module" | "statement_block" | "program" => {
                let mut c = p.walk();
                let before: Vec<Node> = p
                    .named_children(&mut c)
                    .take_while(|k| k.end_byte() <= child.start_byte())
                    .collect();
                for s in before.iter().rev() {
                    if let Some((ty, value)) = binding(*s, name, src) {
                        return match ty {
                            Some(t) => Some((Base::Type(t), vec![])),
                            None => receiver_dyn(value?, src, depth + 1),
                        };
                    }
                    if rebinds(text(*s), name) {
                        return None;
                    }
                }
            }
            "function_definition"
            | "function_declaration"
            | "function_expression"
            | "function"
            | "method_definition"
            | "arrow_function"
            | "lambda" => {
                let params = p
                    .child_by_field_name("parameters")
                    .or_else(|| p.child_by_field_name("parameter"));
                if params.is_some_and(|x| has_word(text(x), name)) {
                    return param_type(params?, name, src).map(|t| (Base::Type(t), vec![]));
                }
                // Python: a name assigned anywhere in a function is that function's own, never an outer one.
                if p.kind() == "function_definition" && rebinds(text(p), name) {
                    return None;
                }
            }
            // Python comprehensions bind their variable after the expression that uses it.
            "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => {
                let mut c = p.walk();
                if p.named_children(&mut c)
                    .any(|k| k.kind() == "for_in_clause" && has_word(text(k), name))
                {
                    return None;
                }
            }
            "for_statement" | "for_in_statement" | "with_statement" | "except_clause"
            | "catch_clause"
                if has_word(
                    &String::from_utf8_lossy(&src[p.start_byte()..child.start_byte()]),
                    name,
                ) =>
            {
                return None;
            }
            _ => {}
        }
        child = p;
    }
    None
}

/// Trace a Java receiver to a written type: `this`, a local, parameter or field, `new T()` or a cast, followed by fields and
/// method calls. A `var` follows its value; anything else (a lambda parameter, a pattern, an anonymous class) gives up.
fn receiver_java(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    if depth > 12 {
        return None;
    }
    let text = |n: Node| n.utf8_text(src).ok().map(str::to_string);
    let then = |inner: Node, step: Step| {
        let (b, mut s) = receiver_java(inner, src, depth + 1)?;
        s.push(step);
        Some((b, s))
    };
    let me = || (!in_anonymous(e)).then(|| Base::Type("Self".into()));
    match e.kind() {
        "this" => Some((me()?, vec![])),
        "identifier" => declared_java(e, src, depth),
        "field_access" => then(
            e.child_by_field_name("object")?,
            Step::Field(text(e.child_by_field_name("field")?)?),
        ),
        "method_invocation" => {
            let name = text(e.child_by_field_name("name")?)?;
            match e.child_by_field_name("object") {
                Some(o) => then(o, Step::Call(name)),
                None => Some((me()?, vec![Step::Call(name)])),
            }
        }
        "object_creation_expression" if e.child_by_field_name("body").is_none() => {
            let t = text(e.child_by_field_name("type")?)?;
            Some((Base::Type(head_dyn(&t)?), vec![]))
        }
        "cast_expression" => Some((Base::Type(text(e.child_by_field_name("type")?)?), vec![])),
        "parenthesized_expression" => receiver_java(e.named_child(0)?, src, depth + 1),
        _ => None,
    }
}

/// The named Java types around a node, innermost first.
fn enclosing_classes(n: Node, src: &[u8]) -> Vec<String> {
    std::iter::successors(n.parent(), |p| p.parent())
        .filter(|p| {
            matches!(
                p.kind(),
                "class_declaration"
                    | "interface_declaration"
                    | "record_declaration"
                    | "enum_declaration"
            )
        })
        .filter_map(|p| owner_name(&p, src))
        .collect()
}

/// Whether `n` sits in an anonymous class body, where `this` and bare names are no longer the enclosing named class's.
fn in_anonymous(n: Node) -> bool {
    let mut p = n.parent();
    while let Some(x) = p {
        if x.kind() == "class_body"
            && x.parent()
                .is_some_and(|c| matches!(c.kind(), "object_creation_expression" | "enum_constant"))
        {
            return true;
        }
        if matches!(
            x.kind(),
            "class_declaration" | "record_declaration" | "enum_declaration"
        ) {
            return false;
        }
        p = x.parent();
    }
    false
}

/// Where a Java name gets its type before `e`: a local declared in an enclosing block, a for-each variable, a parameter, or
/// a field of the enclosing class. Anything that may rebind it in between gives up.
fn declared_java(e: Node, src: &[u8], depth: u8) -> Option<(Base, Vec<Step>)> {
    let name = e.utf8_text(src).ok()?;
    let text = |n: Node| n.utf8_text(src).unwrap_or("");
    let typed = |t: Node, value: Option<Node>| {
        let ty = text(t).trim();
        if ty == "var" {
            receiver_java(value?, src, depth + 1)
        } else {
            Some((Base::Type(ty.to_string()), vec![]))
        }
    };
    let mut anon = false;
    let mut child = e;
    while let Some(p) = child.parent() {
        match p.kind() {
            "block" | "constructor_body" | "switch_block_statement_group" => {
                let mut c = p.walk();
                let before: Vec<Node> = p
                    .named_children(&mut c)
                    .take_while(|k| k.end_byte() <= child.start_byte())
                    .collect();
                for s in before.iter().rev() {
                    if s.kind() == "local_variable_declaration" {
                        let mut c = s.walk();
                        let d = s.named_children(&mut c).find(|d| {
                            d.kind() == "variable_declarator"
                                && d.child_by_field_name("name").map(text) == Some(name)
                        });
                        if let Some(d) = d {
                            return typed(
                                s.child_by_field_name("type")?,
                                d.child_by_field_name("value"),
                            );
                        }
                    }
                    let t = text(*s);
                    if (t.contains("instanceof") && has_word(t, name)) || assigns(t, name) {
                        return None;
                    }
                }
            }
            "method_declaration" | "constructor_declaration" => {
                if let Some(params) = p.child_by_field_name("parameters") {
                    let mut c = params.walk();
                    let found = params.named_children(&mut c).find(|k| {
                        matches!(k.kind(), "formal_parameter" | "spread_parameter")
                            && k.child_by_field_name("name").map(text) == Some(name)
                    });
                    if let Some(k) = found {
                        return typed(k.child_by_field_name("type")?, None)
                            .filter(|_| k.kind() == "formal_parameter");
                    }
                }
            }
            "lambda_expression"
                if p.child_by_field_name("parameters")
                    .is_some_and(|k| has_word(text(k), name)) =>
            {
                return None;
            }
            "enhanced_for_statement" if p.child_by_field_name("name").map(text) == Some(name) => {
                let in_body = p
                    .child_by_field_name("body")
                    .is_some_and(|b| b.id() == child.id());
                return in_body
                    .then(|| typed(p.child_by_field_name("type")?, None))
                    .flatten();
            }
            "for_statement" | "catch_clause" | "try_with_resources_statement"
                if has_word(
                    &String::from_utf8_lossy(&src[p.start_byte()..child.start_byte()]),
                    name,
                ) =>
            {
                return None;
            }
            "class_body"
                if p.parent().is_some_and(|c| {
                    matches!(c.kind(), "object_creation_expression" | "enum_constant")
                }) =>
            {
                anon = true;
            }
            "class_declaration"
            | "record_declaration"
            | "enum_declaration"
            | "interface_declaration" => {
                return (!anon).then(|| {
                    (
                        Base::Type("Self".into()),
                        vec![Step::Field(name.to_string())],
                    )
                });
            }
            _ => {}
        }
        child = p;
    }
    None
}

/// Python, TypeScript and JavaScript inheritance as (class, super): `class A(B)`, `extends B`, `implements C`.
fn class_supers(root: Node, src: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if let Some(class) = owner_name(&n, src).filter(|_| n.kind() != "impl_item") {
            let mut c = n.walk();
            // Python names its bases by field (an argument list); the others by node kind.
            let kids = n
                .children(&mut c)
                .filter(|k| matches!(k.kind(), "class_heritage" | "extends_type_clause"));
            for k in n
                .child_by_field_name("superclasses")
                .into_iter()
                .chain(kids)
            {
                let mut ids = vec![k];
                while let Some(x) = ids.pop() {
                    match x.kind() {
                        "identifier"
                        | "type_identifier"
                        | "attribute"
                        | "member_expression"
                        | "nested_type_identifier" => {
                            if let Ok(t) = x.utf8_text(src) {
                                out.push((class.clone(), t.to_string()));
                            }
                        }
                        "type_arguments" | "arguments" | "keyword_argument" => {}
                        // `Generic[T]`: the base, not its parameters.
                        "subscript" => ids.extend(x.child_by_field_name("value")),
                        _ => {
                            let mut c = x.walk();
                            ids.extend(x.named_children(&mut c));
                        }
                    }
                }
            }
        }
        let mut c = n.walk();
        stack.extend(n.named_children(&mut c));
    }
    out
}

/// Java inheritance as (class, super): what each class or interface extends or implements, generics dropped.
fn java_supers(root: Node, src: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        let decl = matches!(
            n.kind(),
            "class_declaration"
                | "interface_declaration"
                | "record_declaration"
                | "enum_declaration"
        );
        {
            if let Some(class) = decl.then(|| owner_name(&n, src)).flatten() {
                let mut c = n.walk();
                for k in n.children(&mut c).filter(|k| {
                    matches!(
                        k.kind(),
                        "superclass" | "super_interfaces" | "extends_interfaces"
                    )
                }) {
                    let mut ids = vec![k];
                    while let Some(x) = ids.pop() {
                        match x.kind() {
                            "type_identifier" | "scoped_type_identifier" => {
                                if let Ok(t) = x.utf8_text(src) {
                                    out.push((class.clone(), t.to_string()));
                                }
                            }
                            "type_arguments" => {}
                            _ => {
                                let mut c = x.walk();
                                ids.extend(x.named_children(&mut c));
                            }
                        }
                    }
                }
            }
        }
        let mut c = n.walk();
        stack.extend(n.named_children(&mut c));
    }
    out
}

/// Whether `t` assigns `name` (`name = v`, `name += v`; not `name == v`).
fn assigns(t: &str, name: &str) -> bool {
    word_at(t, name).into_iter().any(|i| {
        let after = t[i + name.len()..].trim_start();
        (after.starts_with('=') && !after.starts_with("=="))
            || after
                .strip_prefix(['+', '-', '*', '/', '|', '&', '%', '^'])
                .is_some_and(|r| r.starts_with('='))
    })
}

/// Java field types as (class, field, type): `Repo repo;` in a class body and a record's components.
fn java_fields(root: Node, src: &[u8]) -> Vec<(String, String, String)> {
    let text = |n: Node| n.utf8_text(src).unwrap_or("").to_string();
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "class_declaration" | "record_declaration") {
            let cls = owner_name(&n, src).unwrap_or_default();
            if let Some(body) = n.child_by_field_name("body") {
                let mut c = body.walk();
                for f in body
                    .named_children(&mut c)
                    .filter(|f| f.kind() == "field_declaration")
                {
                    let Some(ty) = f.child_by_field_name("type") else {
                        continue;
                    };
                    let mut k = f.walk();
                    for d in f
                        .named_children(&mut k)
                        .filter(|d| d.kind() == "variable_declarator")
                    {
                        if let Some(name) = d.child_by_field_name("name") {
                            out.push((cls.clone(), text(name), text(ty)));
                        }
                    }
                }
            }
            if let Some(params) = n.child_by_field_name("parameters") {
                let mut c = params.walk();
                for p in params
                    .named_children(&mut c)
                    .filter(|p| p.kind() == "formal_parameter")
                {
                    if let (Some(name), Some(ty)) =
                        (p.child_by_field_name("name"), p.child_by_field_name("type"))
                    {
                        out.push((cls.clone(), text(name), text(ty)));
                    }
                }
            }
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    out
}

/// A statement that declares `name` directly: `x = v`, `x: T = v` (Python), `const x: T = v` (TypeScript). Gives the
/// written type, or the value to trace.
fn binding<'a>(s: Node<'a>, name: &str, src: &[u8]) -> Option<(Option<String>, Option<Node<'a>>)> {
    let decl = |d: Node<'a>, value: &str| {
        let ty = d
            .child_by_field_name("type")
            .and_then(|t| t.utf8_text(src).ok())
            .map(|t| t.trim_start_matches(':').trim().to_string());
        (ty, d.child_by_field_name(value))
    };
    match s.kind() {
        "expression_statement" => {
            let a = s.named_child(0).filter(|a| a.kind() == "assignment")?;
            let left = a.child_by_field_name("left")?;
            (left.kind() == "identifier" && left.utf8_text(src).ok()? == name)
                .then(|| decl(a, "right"))
        }
        "lexical_declaration" | "variable_declaration" => {
            let mut c = s.walk();
            let d = s.named_children(&mut c).find(|d| {
                d.kind() == "variable_declarator"
                    && d.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(src).ok())
                        == Some(name)
            })?;
            Some(decl(d, "value"))
        }
        _ => None,
    }
}

/// The annotation of parameter `name`, if it has one.
fn param_type(params: Node, name: &str, src: &[u8]) -> Option<String> {
    let text = |n: Option<Node>| n.and_then(|n| n.utf8_text(src).ok());
    let mut c = params.walk();
    let p = params.named_children(&mut c).find(|p| {
        let n = p
            .child_by_field_name("name")
            .or_else(|| p.child_by_field_name("pattern"))
            .or_else(|| {
                (p.kind() == "typed_parameter")
                    .then(|| p.named_child(0))
                    .flatten()
            })
            .or(Some(*p));
        text(n) == Some(name)
    })?;
    Some(
        text(p.child_by_field_name("type"))?
            .trim_start_matches(':')
            .trim()
            .to_string(),
    )
}

/// Whether `t` may assign `name`: `name =` (not `==`), `name +=`, `name:`, `for name`, `as name`, `let/const/var name`.
fn rebinds(t: &str, name: &str) -> bool {
    word_at(t, name).into_iter().any(|i| {
        let after = t[i + name.len()..].trim_start();
        let before = t[..i].trim_end();
        (after.starts_with('=') && !after.starts_with("=="))
            || after.starts_with(':')
            || after
                .strip_prefix(['+', '-', '*', '/', '|', '&', '%'])
                .is_some_and(|r| r.starts_with('='))
            || after.starts_with(',')
            || [
                "for", "as", "let", "const", "var", "def", "class", "import", ",",
            ]
            .iter()
            .any(|w| before.ends_with(w))
    })
}

/// The project's files, relative with `/`: what git tracks or would track (so `.gitignore` is honoured), or, outside a git
/// repository or without git, a walk that skips dependency and build folders.
pub(crate) fn candidates(root: &Path) -> Vec<String> {
    let git = std::process::Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "-z"])
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    if let Some(out) = git
        .ok()
        .filter(|o| o.status.success() && !o.stdout.is_empty())
    {
        return out
            .stdout
            .split(|&b| b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();
    }
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Dot folders too: build copies like `.next-qloop-heavy` held most of a 43 MB copy of BorgInvestor (#246).
            !(e.file_type().is_dir()
                && e.depth() > 0
                && (name.starts_with('.') || SKIP.contains(&name.as_ref())))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            e.path()
                .strip_prefix(root)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// The project path a relative import points at, without extension: `relative("src/a/b.ts", "../c.js")` is `src/c`.
fn relative(from: &str, module: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop(); // the importing file itself
    let module = ["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"]
        .iter()
        .find_map(|e| module.strip_suffix(&format!(".{e}")))
        .unwrap_or(module);
    for seg in module.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

fn first(m: &str) -> &str {
    m.split(['/', '.', ':'])
        .find(|s| !s.is_empty())
        .unwrap_or("")
}

/// The qualifier of a call (the object in `obj.f()`, the path in `a::f()`, as its last plain name) and whether the call is on
/// a value (`.`) rather than through a path.
fn qualifier(call: &Node, name: &Node, src: &[u8]) -> (String, bool) {
    // Java keeps the receiver of `obj.method()` in the `object` field.
    if let Some(o) = call.child_by_field_name("object") {
        return (o.utf8_text(src).map_or(String::new(), last_word), true);
    }
    let Some(f) = call
        .child_by_field_name("function")
        .or_else(|| call.named_child(0))
    else {
        return (String::new(), false);
    };
    let text = f.utf8_text(src).unwrap_or("");
    let name = name.utf8_text(src).unwrap_or("");
    let Some(rest) = text.strip_suffix(name).filter(|r| !r.is_empty()) else {
        return (String::new(), false);
    };
    let member = rest.ends_with('.') || rest.ends_with("?.") || rest.ends_with("->");
    // A computed receiver (`new Hono().get()`, `f()?.g()`, `a[0].f()`) has no name to resolve through.
    if member && rest.contains([')', ']']) {
        return (String::new(), true);
    }
    (
        last_word(rest.trim_end_matches(['.', ':', '?', '!', '-', '>'])),
        member,
    )
}
fn last_word(s: &str) -> String {
    s.rsplit(['.', ':', '(', ')', ' '])
        .find(|w| !w.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Names an import statement binds, each with the module it comes from.
fn bindings(l: Lang, text: &str) -> Vec<(String, String)> {
    let text = text.trim().trim_end_matches(';');
    match l {
        Lang::Rust => rust_use(text),
        Lang::Ts | Lang::Tsx | Lang::Js => {
            let Some(q) = text.find(['"', '\'']) else {
                return vec![];
            };
            let module = text[q + 1..].trim_end_matches(['"', '\'']).to_string();
            let clause = text[..q]
                .trim()
                .trim_start_matches("import")
                .trim_start_matches("type")
                .trim()
                .trim_end_matches("from")
                .trim();
            clause
                .replace(['{', '}'], ",")
                .split(',')
                .filter_map(|p| {
                    let p = p.trim();
                    let bound = p
                        .rsplit(" as ")
                        .next()?
                        .trim()
                        .trim_start_matches("* ")
                        .trim();
                    (!bound.is_empty() && bound != "*").then(|| (bound.to_string(), module.clone()))
                })
                .collect()
        }
        Lang::Python => {
            if let Some(rest) = text.strip_prefix("from ") {
                let Some((module, names)) = rest.split_once(" import ") else {
                    return vec![];
                };
                names
                    .trim_matches(['(', ')', ' '])
                    .split(',')
                    .filter_map(|n| {
                        let n = n.trim();
                        let bound = n.rsplit(" as ").next()?.trim();
                        (!bound.is_empty() && bound != "*")
                            .then(|| (bound.to_string(), module.trim().to_string()))
                    })
                    .collect()
            } else {
                text.trim_start_matches("import ")
                    .split(',')
                    .filter_map(|n| {
                        let n = n.trim();
                        let (module, bound) =
                            n.split_once(" as ").map_or((n, first(n)), |(m, b)| (m, b));
                        (!module.is_empty())
                            .then(|| (bound.trim().to_string(), module.trim().to_string()))
                    })
                    .collect()
            }
        }
        Lang::Java => {
            let path = text
                .trim_start_matches("import")
                .trim()
                .trim_start_matches("static ")
                .trim();
            let bound = path.rsplit('.').next().unwrap_or(path);
            if bound == "*" {
                vec![]
            } else {
                vec![(bound.to_string(), path.to_string())]
            }
        }
        Lang::Go => {
            let Some(q) = text.find('"') else {
                return vec![];
            };
            let module = text[q + 1..].trim_end_matches('"').to_string();
            let alias = text[..q].trim();
            let bound = if alias.is_empty() {
                module.rsplit('/').next().unwrap_or(&module).to_string()
            } else {
                alias.to_string()
            };
            vec![(bound, module)]
        }
    }
}

/// `a::b::{c, d as e, f::g}` -> (c, a::b), (e, a::b), (g, a::b::f); `a::b` -> (b, a).
fn rust_use(text: &str) -> Vec<(String, String)> {
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some((base, rest)) = text.split_once("::{") {
        return rest
            .trim_end_matches('}')
            .split(',')
            .flat_map(|item| {
                let item = item.trim();
                if item.is_empty() || item == "self" || item.contains('{') {
                    return vec![];
                }
                rust_use(&format!("{base}::{item}"))
            })
            .collect();
    }
    let (path, alias) = text
        .split_once(" as ")
        .map_or((text.as_str(), None), |(p, a)| (p, Some(a.trim())));
    let Some((module, name)) = path.rsplit_once("::") else {
        return vec![];
    };
    if name == "*" {
        return vec![];
    }
    vec![(alias.unwrap_or(name).to_string(), module.to_string())]
}

/// The outside package a module belongs to, as its manager names it.
fn package_of(l: Lang, module: &str) -> String {
    match l {
        Lang::Ts | Lang::Tsx | Lang::Js if module.starts_with('@') => {
            module.splitn(3, '/').take(2).collect::<Vec<_>>().join("/")
        }
        Lang::Go if module.contains('.') => {
            module.splitn(4, '/').take(3).collect::<Vec<_>>().join("/")
        }
        Lang::Java => module.splitn(3, '.').take(2).collect::<Vec<_>>().join("."),
        _ => first(module).replace('-', "_"),
    }
    .replace(' ', "_")
}

const PY_STDLIB: &str = "abc argparse array ast asyncio atexit base64 binascii bisect builtins bz2 calendar cmath cmd code codecs collections colorsys concurrent configparser contextlib contextvars copy csv ctypes curses dataclasses datetime dbm decimal difflib dis doctest email encodings enum errno faulthandler fcntl filecmp fileinput fnmatch fractions ftplib functools gc getopt getpass gettext glob graphlib grp gzip hashlib heapq hmac html http imaplib importlib inspect io ipaddress itertools json keyword linecache locale logging lzma mailbox marshal math mimetypes mmap msvcrt multiprocessing netrc numbers operator optparse os pathlib pdb pickle pkgutil platform plistlib poplib posixpath pprint profile pstats pty pwd queue random re readline reprlib resource runpy sched secrets select selectors shelve shlex shutil signal site smtplib socket socketserver sqlite3 ssl stat statistics string struct subprocess sys sysconfig syslog tarfile tempfile termios textwrap threading time timeit tkinter token tokenize tomllib trace traceback tracemalloc tty turtle types typing unicodedata unittest urllib uuid venv warnings wave weakref webbrowser winreg wsgiref xml xmlrpc zipapp zipfile zipimport zlib zoneinfo __future__";
const NODE_BUILTINS: &str = "assert async_hooks buffer child_process cluster console constants crypto dgram diagnostics_channel dns domain events fs http http2 https inspector module net os path perf_hooks process punycode querystring readline repl stream string_decoder sys timers tls trace_events tty url util v8 vm wasi worker_threads zlib";

/// The outside packages the project's source imports, as (language, package name as its manager names it), for the stack
/// detector (#304). Standard library and the project's own modules are left out; Java is not covered. Same parser and the
/// same notion of "local" as the code graph.
pub fn imports(root: &Path) -> Vec<(&'static str, String)> {
    let all = candidates(root);
    let mut files: Vec<(Lang, Vec<u8>)> = Vec::new();
    for rel in &all {
        let Some(l) = lang_of(rel).filter(|l| *l != Lang::Java) else {
            continue;
        };
        let path = root.join(rel);
        if files.len() < MAX_FILES
            && std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE)
            && let Ok(b) = std::fs::read(&path)
        {
            files.push((l, b));
        }
    }
    let stems: HashSet<String> = all
        .iter()
        .flat_map(|p| {
            p.split('/')
                .map(|s| s.split('.').next().unwrap_or(s).replace('-', "_"))
        })
        .collect();
    // The project's own names: Rust workspace crates (as code spells them) and the Go module path.
    let own: HashSet<String> = all
        .iter()
        .filter(|p| p.ends_with("Cargo.toml") || p.ends_with("go.mod"))
        .filter_map(|p| std::fs::read_to_string(root.join(p)).ok())
        .flat_map(|t| {
            t.lines()
                .filter_map(|l| {
                    let l = l.trim();
                    if let Some(m) = l.strip_prefix("module ") {
                        return Some(m.trim().to_string());
                    }
                    let (k, v) = l.split_once('=')?;
                    (k.trim() == "name").then(|| v.trim().trim_matches('"').replace('-', "_"))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let js = tree_sitter_javascript::LANGUAGE.into();
    let require = Query::new(
        &js,
        "(call_expression function: (identifier) @f arguments: (arguments . (string) @s))",
    )
    .ok();
    let mut found: std::collections::BTreeSet<(&'static str, String)> = Default::default();
    let mut parser = Parser::new();
    for (l, src) in &files {
        let (lang, _, imp) = grammar(*l);
        let Ok(q) = Query::new(&lang, imp) else {
            continue;
        };
        if parser.set_language(&lang).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(src, None) else {
            continue;
        };
        let mut modules: Vec<String> = Vec::new();
        let mut cur = QueryCursor::new();
        let mut it = cur.matches(&q, tree.root_node(), src.as_slice());
        while let Some(m) = it.next() {
            for c in m.captures() {
                if let Ok(text) = c.node.utf8_text(src) {
                    modules.extend(bindings(*l, text).into_iter().map(|(_, m)| m));
                }
            }
        }
        // CommonJS `require("x")`; the JavaScript grammar parses a TypeScript file's calls the same way.
        if let (Some(rq), Lang::Js | Lang::Ts | Lang::Tsx) = (&require, *l)
            && parser.set_language(&js).is_ok()
            && let Some(tree) = parser.parse(src, None)
        {
            let mut cur = QueryCursor::new();
            let mut it = cur.matches(rq, tree.root_node(), src.as_slice());
            while let Some(m) = it.next() {
                let t: Vec<&str> = m
                    .captures()
                    .iter()
                    .map(|c| c.node.utf8_text(src).unwrap_or(""))
                    .collect();
                if let [f, s] = t[..]
                    && f == "require"
                {
                    modules.push(s.trim_matches(['"', '\'', '`']).to_string());
                }
            }
        }
        for m in modules {
            let head = first(&m);
            let outside = !m.is_empty()
                && !m.starts_with(['.', '/', '~', '#'])
                && !m.starts_with("@/")
                && !stems.contains(&head.replace('-', "_"));
            let (name, outside) = match l {
                Lang::Rust => (
                    "rust",
                    outside
                        && ![
                            "crate",
                            "self",
                            "super",
                            "std",
                            "core",
                            "alloc",
                            "proc_macro",
                            "test",
                        ]
                        .contains(&head)
                        && !own.contains(&head.replace('-', "_")),
                ),
                Lang::Python => (
                    "python",
                    outside && !PY_STDLIB.split(' ').any(|s| s == head),
                ),
                Lang::Go => (
                    "go",
                    outside
                        && m.split('/').next().is_some_and(|h| h.contains('.'))
                        && !own.iter().any(|o| m.starts_with(o.as_str())),
                ),
                _ => (
                    "javascript",
                    outside
                        && !m.starts_with("node:")
                        && !NODE_BUILTINS.split(' ').any(|s| s == head),
                ),
            };
            if outside {
                found.insert((name, package_of(*l, &m)));
            }
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_bind_names_to_their_modules() {
        assert_eq!(
            rust_use("crate::store::{Filters, open as o, sub::Deep}"),
            [
                ("Filters".into(), "crate::store".into()),
                ("o".into(), "crate::store".into()),
                ("Deep".into(), "crate::store::sub".into())
            ]
        );
        assert_eq!(
            bindings(Lang::Js, "import express, { Router as R } from 'express'"),
            [
                ("express".into(), "express".into()),
                ("R".into(), "express".into())
            ]
        );
        assert_eq!(
            bindings(Lang::Python, "from .store import open_db, Row as R"),
            [
                ("open_db".into(), ".store".into()),
                ("R".into(), ".store".into())
            ]
        );
        assert_eq!(
            bindings(Lang::Python, "import os.path as p, json"),
            [
                ("p".into(), "os.path".into()),
                ("json".into(), "json".into())
            ]
        );
        assert_eq!(
            bindings(Lang::Go, "\"github.com/acme/kit/log\""),
            [("log".into(), "github.com/acme/kit/log".into())]
        );
        assert_eq!(
            bindings(Lang::Java, "import com.acme.store.Repo;"),
            [("Repo".into(), "com.acme.store.Repo".into())]
        );
        assert_eq!(package_of(Lang::Ts, "@scope/pkg/sub"), "@scope/pkg");
        assert_eq!(relative("src/a/b.ts", "../c.js").as_deref(), Some("src/c"));
        assert_eq!(relative("main.ts", "./ws").as_deref(), Some("ws"));
        assert_eq!(relative("main.ts", "../out"), None); // outside the project
        assert_eq!(
            package_of(Lang::Go, "github.com/acme/kit/log"),
            "github.com/acme/kit"
        );
    }

    #[test]
    fn calls_resolve_through_imports_and_ambiguous_ones_stay_unlinked() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| {
            let f = t.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, s).unwrap();
        };
        w(
            "app/main.py",
            "from .store import save\nimport requests\n\ndef run():\n    save()\n    helper()\n    requests.get('x')\n    load()\n\ndef helper():\n    pass\n",
        );
        w(
            "app/store.py",
            "def save():\n    pass\n\ndef load():\n    pass\n",
        );
        w(
            "app/other.py",
            "def save():\n    pass\n\ndef load():\n    pass\n",
        );
        let (docs, resolved) = index(t.path(), "demo").unwrap();
        let main = docs.iter().find(|d| d.path == "app/main.py").unwrap();
        let refs: Vec<&str> = main
            .occurrences
            .iter()
            .filter(|o| o.roles == 0)
            .map(|o| o.symbol.as_str())
            .collect();
        assert!(
            refs.iter().any(|s| s.contains("app/store.py/save:")),
            "{refs:?}"
        ); // through the import, not other.py
        assert!(refs.iter().any(|s| s.contains("app/main.py/helper:"))); // same file
        assert!(
            refs.iter()
                .any(|s| s.starts_with("treesitter pkg requests . get."))
        ); // outside package
        assert!(!refs.iter().any(|s| s.contains("/load:"))); // two `load`, neither imported: no link
        assert_eq!(resolved, 2);
    }

    #[test]
    fn rust_calls_inside_macro_arguments_are_found() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("src")).unwrap();
        std::fs::write(
            t.path().join("src/lib.rs"),
            "mod store;\nfn helper() -> u32 { 1 }\nfn run(x: Vec<u32>) {\n    assert_eq!(helper(), 1);\n    println!(\"{}\", store::open());\n    println!(\"{}\", x.len());\n}\n",
        )
        .unwrap();
        std::fs::write(
            t.path().join("src/store.rs"),
            "pub fn open() -> u32 { 2 }\n",
        )
        .unwrap();
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut found: Vec<&str> = rules.iter().map(String::as_str).collect();
        found.sort();
        assert_eq!(
            found,
            [
                "src/lib.rs\t4\thelper\tsrc/lib.rs\tsame_file",
                "src/lib.rs\t5\topen\tsrc/store.rs\tmodule_path"
            ],
            "{rules:?}"
        ); // `x.len()` is a method on an unknown type: left out
    }

    #[test]
    fn outside_git_the_walk_skips_dependency_and_dot_folders() {
        let t = tempfile::tempdir().unwrap();
        for p in ["src/a.ts", ".next-qloop/chunk.js", "node_modules/x/i.js"] {
            let f = t.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, "").unwrap();
        }
        assert_eq!(candidates(t.path()), ["src/a.ts"]);
    }

    #[test]
    fn typescript_files_get_definitions_and_calls() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("a.ts"),
            "export class Store {\n  save(x: number): void {\n    this.flush(x);\n  }\n  flush(x: number) {}\n}\nfunction run(s: Store) {\n  helper(1);\n  other.flush(2);\n}\nconst helper = (n: number) => n;\n",
        )
        .unwrap();
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        assert_eq!(
            rules,
            [
                "a.ts\t3\tflush\ta.ts\treceiver_type",
                "a.ts\t8\thelper\ta.ts\tsame_file"
            ],
            "{rules:?}"
        ); // `other.flush(2)` is a call on a value of unknown type: no link
    }

    #[test]
    fn python_and_typescript_method_calls_link_through_the_receiver_type() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w(
            "store.py",
            "class Db:\n    def run(self):\n        pass\n\nclass Store:\n    def __init__(self):\n        self.db = Db()\n\n    def save(self) -> \"Db\":\n        self.db.run()\n        return self.db\n",
        );
        w(
            "other.py",
            "class Other:\n    def save(self):\n        pass\n\n    def run(self):\n        pass\n",
        );
        w(
            "main.py",
            "from store import Store\n\ndef main(o: Other, u):\n    s = Store()\n    s.save().run()\n    o.run()\n    u.save()\n    for s in []:\n        s.save()\n    v = [x.save() for x in []]\n",
        );
        w(
            "app.ts",
            "class Repo {\n  find(): Item { return new Item(); }\n}\nclass Item {\n  open() {}\n}\nclass Svc {\n  constructor(private repo: Repo) {}\n  async load(): Promise<Item> { return this.repo.find(); }\n  async go(r: Repo | null) {\n    this.repo.find().open();\n    (await this.load()).open();\n    const i = new Item();\n    i.open();\n    let j = r;\n    j.find();\n    for (const k of []) { k.open(); }\n  }\n}\n",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut typed: Vec<&str> = rules
            .iter()
            .map(String::as_str)
            .filter(|l| l.ends_with("receiver_type"))
            .collect();
        typed.sort();
        assert_eq!(
            typed,
            [
                "app.ts\t11\tfind\tapp.ts\treceiver_type",
                "app.ts\t11\topen\tapp.ts\treceiver_type",
                "app.ts\t12\tload\tapp.ts\treceiver_type",
                "app.ts\t12\topen\tapp.ts\treceiver_type",
                "app.ts\t14\topen\tapp.ts\treceiver_type",
                "app.ts\t16\tfind\tapp.ts\treceiver_type",
                "app.ts\t9\tfind\tapp.ts\treceiver_type",
                "main.py\t5\trun\tstore.py\treceiver_type",
                "main.py\t5\tsave\tstore.py\treceiver_type",
                "main.py\t6\trun\tother.py\treceiver_type",
                "store.py\t10\trun\tstore.py\treceiver_type",
            ],
            "{rules:?}"
        ); // `u.save()` (no annotation), loop and comprehension variables: no link
    }

    #[test]
    fn java_overloads_are_told_apart_by_argument_count() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w(
            "A.java",
            "class Util {
  static void log(String a) {}
  static void log(String a, int b) {}
  static void same(int a) {}
  static void same(long a) {}
  static void rest(String... a) {}
  static void rest(String a, int b) {}
}
class Use {
  void go(Util u) {
    u.log(\"a\");
    u.log(\"a\", 1);
    u.same(1);
    u.rest(\"a\", 1);
  }
}
",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut typed: Vec<&str> = rules.iter().map(String::as_str).collect();
        typed.sort();
        // Two `same` of one arity and a varargs `rest` stay unlinked.
        assert_eq!(
            typed,
            [
                "A.java	11	log	A.java	receiver_type",
                "A.java	12	log	A.java	receiver_type"
            ],
            "{rules:?}"
        );
    }

    #[test]
    fn java_calls_reach_methods_inherited_from_a_superclass_or_interface() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w(
            "A.java",
            "interface Named {
  String name();
}
class Base implements Named {
  public String name() { return \"\"; }
  void open() {}
}
class Mid extends Base {}
class Child extends Mid {
  void go(Child c) {
    open();
    this.name();
    c.open();
  }
}
class Loose extends Elsewhere {
  void go() { open(); }
}
",
        );
        w(
            "B.java",
            "class Other {
  void open() {}
}
",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut got: Vec<&str> = rules.iter().map(String::as_str).collect();
        got.sort();
        // `Loose extends Elsewhere` (outside the project): its `open()` may be Elsewhere's, so no link.
        assert_eq!(
            got,
            [
                "A.java	11	open	A.java	receiver_type",
                "A.java	12	name	A.java	receiver_type",
                "A.java	13	open	A.java	receiver_type",
            ],
            "{rules:?}"
        );
    }

    #[test]
    fn python_and_typescript_calls_reach_inherited_methods() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w(
            "base.py",
            "class Base:
    def open(self):
        pass
",
        );
        w(
            "a.py",
            "from base import Base

class Child(Base):
    def go(self):
        self.open()

class Loose(Elsewhere):
    def go(self):
        self.open()
",
        );
        w(
            "base.ts",
            "export class Root { open() {} }
export interface Named { name(): string }
",
        );
        w(
            "b.ts",
            "import { Root, Named } from './base';
class Kid extends Root implements Named {
  name() { return ''; }
  go() { this.open(); }
}
class Fld extends Root {
  open: () => void = () => {};
  go() { this.open(); }
}
class Wild extends Elsewhere {
  go() { this.open(); }
}
",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut got: Vec<&str> = rules.iter().map(String::as_str).collect();
        got.sort();
        // `Loose` extends something outside the project: its `open()` may be that class's, so no link.
        assert_eq!(
            got,
            [
                "a.py	5	open	base.py	receiver_type",
                "b.ts	4	open	base.ts	receiver_type",
            ],
            "{got:?}"
        );
    }

    #[test]
    fn java_standard_library_imports_stay_outside_the_project() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("src/main/java/acme")).unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        // A project class named like `java.util.Set`, and a folder named `java` that must not make the import local.
        w(
            "src/main/java/acme/Set.java",
            "package acme;
class Set {
  void add(Object o) {}
}
",
        );
        w(
            "src/main/java/acme/C.java",
            "package acme;
import java.util.Set;
class C {
  private Set<String> s;
  void f() { s.add(\"x\"); }
}
",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        assert!(
            !rules.iter().any(|l| l.contains("receiver_type")),
            "{rules:?}"
        );
    }

    #[test]
    fn java_method_calls_link_through_the_receiver_type() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w(
            "A.java",
            "class Repo {\n  Item find() { return new Item(); }\n}\nclass Item {\n  void open() {}\n}\nclass Svc {\n  private Repo repo;\n  void go(Repo r, Object u) {\n    repo.find().open();\n    this.repo.find();\n    var i = new Item();\n    i.open();\n    r.find();\n    u.open();\n    for (Item k : items()) { k.open(); }\n    Runnable f = () -> r.find();\n  }\n}\n",
        );
        w(
            "B.java",
            "class Other {\n  void open() {}\n  Item find() { return null; }\n}\n",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut typed: Vec<&str> = rules
            .iter()
            .map(String::as_str)
            .filter(|l| l.ends_with("receiver_type"))
            .collect();
        typed.sort();
        assert_eq!(
            typed,
            [
                "A.java\t10\tfind\tA.java\treceiver_type",
                "A.java\t10\topen\tA.java\treceiver_type",
                "A.java\t11\tfind\tA.java\treceiver_type",
                "A.java\t13\topen\tA.java\treceiver_type",
                "A.java\t14\tfind\tA.java\treceiver_type",
                "A.java\t16\topen\tA.java\treceiver_type",
                "A.java\t17\tfind\tA.java\treceiver_type",
            ],
            "{rules:?}"
        ); // `u.open()` (Object): no link, though `Other.open` exists
    }

    #[test]
    fn a_rust_type_path_links_only_to_that_types_own_method() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("src")).unwrap();
        std::fs::write(
            t.path().join("src/lib.rs"),
            "#[derive(Default)]\npub struct ByteBuf;\npub struct Phase;\nimpl Default for Phase {\n    fn default() -> Self {\n        Phase\n    }\n}\nfn run() {\n    let a = ByteBuf::default();\n    let b = Phase::default();\n    default();\n}\n",
        )
        .unwrap();
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let typed: Vec<&str> = rules
            .iter()
            .map(String::as_str)
            .filter(|l| l.ends_with("type_path"))
            .collect();
        assert_eq!(
            typed,
            ["src/lib.rs\t11\tdefault\tsrc/lib.rs\ttype_path"],
            "{rules:?}"
        );
        // A bare `default()` is not `Phase::default`: no link.
        assert!(!rules.iter().any(|l| l.contains("\t12\t")), "{rules:?}");
    }

    #[test]
    fn rust_method_calls_link_through_the_receiver_type() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| {
            let f = t.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, s).unwrap();
        };
        w(
            "src/store.rs",
            "pub struct Db;\nimpl Db {\n    pub fn run(&self) {}\n}\npub struct Store {\n    db: Db,\n}\nimpl Store {\n    pub fn open() -> anyhow::Result<Self> {\n        todo!()\n    }\n    pub fn save(&self) {\n        self.db.run();\n    }\n}\n",
        );
        w(
            "src/other.rs",
            "pub struct Other;\nimpl Other {\n    pub fn save(&self) {}\n    pub fn run(&self) {}\n}\n",
        );
        w(
            "src/main.rs",
            "fn main() {\n    let s = Store::open().unwrap();\n    s.save();\n    let s = Store::open()?;\n    s.db.run();\n    let f = |s: Other| s.save();\n    let v = Vec::new();\n    v.save();\n}\n\nfn with(o: &Other) {\n    o.run();\n}\n",
        );
        let (_, _, rules) = parse(t.path(), "demo").unwrap();
        let mut got: Vec<&str> = rules
            .iter()
            .map(String::as_str)
            .filter(|l| l.ends_with("receiver_type"))
            .collect();
        got.sort();
        assert_eq!(
            got,
            [
                "src/main.rs\t12\trun\tsrc/other.rs\treceiver_type", // parameter `o: &Other`
                "src/main.rs\t3\tsave\tsrc/store.rs\treceiver_type", // `Store::open().unwrap()`
                "src/main.rs\t5\trun\tsrc/store.rs\treceiver_type", // shadowed `s`, `?`, then field `db: Db`
                "src/store.rs\t13\trun\tsrc/store.rs\treceiver_type", // `self.db`
            ],
            "{rules:?}"
        );
    }

    #[test]
    fn receiver_tables_abstain_on_ambiguous_and_external_types_and_follow_custom_returns() {
        let t = tempfile::tempdir().unwrap();
        let w = |p: &str, s: &str| std::fs::write(t.path().join(p), s).unwrap();
        w("a.rs", "struct Same; impl Same { fn save(&self) {} }");
        w("b.rs", "struct Same; impl Same { fn other(&self) {} }");
        w(
            "store.rs",
            "struct Store; impl Store { fn save(&self) {} fn clone(&self) -> Other { todo!() } } struct Other; impl Other { fn save(&self) {} }",
        );
        w(
            "caller.rs",
            "use dependency::Store; fn f(s: &Store, x: &Same) { s.save(); x.save(); } fn g(s: &dependency::Other) { s.save(); }",
        );
        w("custom.rs", "fn f(s: &Store) { s.clone().save(); }");
        w(
            "unknown.rs",
            "struct Unknown; impl Unknown { fn save(&self) {} } fn f(s: &Unknown) { s.borrow().save(); }",
        );
        let (_, _, rules) = parse(t.path(), "fixture").unwrap();
        let typed: Vec<_> = rules
            .iter()
            .filter(|r| r.ends_with("receiver_type"))
            .collect();
        assert_eq!(typed.len(), 2, "{rules:?}");
        assert!(
            typed
                .iter()
                .any(|r| r.starts_with("custom.rs\t1\tsave\tstore.rs")),
            "{rules:?}"
        );
        assert!(
            typed
                .iter()
                .any(|r| r.starts_with("custom.rs\t1\tclone\tstore.rs")),
            "{rules:?}"
        );
    }
}
