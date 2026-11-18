//! The tree-sitter base layer (CODE-GRAPH-DESIGN.md section 12): definitions, calls and imports parsed inside the binary for
//! every supported language, with no outside indexer and no build. A call is linked only when it resolves: to a definition in
//! the same file, through the file's imports (a project module or an outside package), or to a name defined once in the
//! project. An ambiguous call gets no link: precision over recall. Links made here are graded `resolved`, not `compiler`.
//! The output has the same shape as a SCIP index, so the graph, lookups and the map need nothing new.
use crate::scip::{DEFINITION, Document, Occurrence, Symbol};
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
    file: usize,
    range: [i32; 4],
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
    let mut compiled: HashMap<Lang, (Language, Query, Query)> = HashMap::new();
    // Rust struct field types, (struct, field) -> types: one step of a receiver chain (`self.store.save()`).
    let mut fields: HashMap<(String, String), Vec<(String, usize)>> = HashMap::new();
    let rust_fields = Query::new(&tree_sitter_rust::LANGUAGE.into(), RUST_FIELDS).ok();
    let mut parser = Parser::new();
    for (i, (_, l, src)) in files.iter().enumerate() {
        if !compiled.contains_key(l) {
            let (lang, tags, imp) = grammar(*l);
            // The grammar's tags query plus calls it leaves out: Rust path calls (`a::f()`), measured as the biggest gap.
            let tags = if *l == Lang::Rust {
                format!("{tags}\n{RUST_PATH_CALLS}")
            } else {
                tags.to_string()
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
                let (owner, ret) = if *l == Lang::Rust && kind == "function" {
                    let owner = node
                        .parent()
                        .filter(|p| p.kind() == "declaration_list")
                        .and_then(|p| p.parent())
                        .and_then(|p| owner_name(&p, src));
                    let ret = node
                        .child_by_field_name("return_type")
                        .and_then(|r| r.utf8_text(src).ok())
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
                });
            } else if node.kind() != "macro_invocation" {
                // A Rust macro (`env!`, `write!`) is not a call to a function of that name.
                let (qualifier, member) = qualifier(&node, &n, src);
                let recv = if *l == Lang::Rust && member {
                    node.child_by_field_name("function")
                        .and_then(|f| f.child_by_field_name("value"))
                        .and_then(|v| receiver(v, src, 0))
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
                calls.push(Call {
                    name: text.into(),
                    qualifier,
                    member,
                    recv,
                    file: i,
                    range: range(&n),
                });
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
        let mut cur = QueryCursor::new();
        let mut it = cur.matches(imp, tree.root_node(), src.as_slice());
        while let Some(m) = it.next() {
            for c in m.captures() {
                if let Ok(text) = c.node.utf8_text(src) {
                    for (bound, module) in bindings(*l, text) {
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
    let local = |m: &str| {
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
        // Qualified types need module resolution; stripping their path could bind a dependency to our own type.
        if ty.contains("::") {
            return None;
        }
        let owner = head(ty);
        if imports[file].contains_key(&owner) {
            return None; // imported type identity is not established by this table
        }
        // Equal short names in different modules are distinct types. Never combine their method tables.
        let types = by_name.get(owner.as_str())?;
        if types.iter().filter(|&&d| defs[d].kind == "type").count() != 1 {
            return None;
        }
        Some(owner)
    };
    let method = |ty: &str, name: &str, file: usize| {
        let owner = owner(ty, file)?;
        match methods.get(&(owner.as_str(), name)) {
            Some(v) if v.len() == 1 => Some(v[0]),
            _ => None,
        }
    };
    // The type a receiver chain ends on; `None` as soon as one step is unknown (std, a dependency, a generic).
    let type_of = |b: &Base, steps: &[Step], file: usize| -> Option<(String, usize)> {
        let (mut t, mut context) = match b {
            Base::Type(t) => (t.clone(), file),
            Base::Static(ty, f) => {
                let d = &defs[method(ty, f, file)?];
                (d.ret.clone()?, d.file)
            }
        };
        for s in steps {
            t = match s {
                Step::Try => unwrap_ty(&t)?,
                Step::Call(m) if m == "unwrap" || m == "expect" => unwrap_ty(&t)?,
                Step::Call(m) => {
                    let d = &defs[method(&t, m, context)?];
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
                    .any(|seg| seg.split('.').next() == Some(last))
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
        });
    }
    let mut resolved = 0;
    let mut rules: Vec<String> = Vec::new();
    for c in &calls {
        let cands = by_name
            .get(c.name.as_str())
            .map_or(&[][..], |v| v.as_slice());
        let same: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&d| defs[d].file == c.file)
            // A bare Rust call (`write_help(..)`) never reaches a method; clap's `Command::write_help` caught this.
            .filter(|&d| !(c.qualifier.is_empty() && defs[d].owner.is_some()))
            .collect();
        let via = if c.qualifier.is_empty() {
            imports[c.file].get(&c.name)
        } else {
            imports[c.file].get(&c.qualifier)
        };
        // A name the calling item binds itself (a closure in a `let`, a parameter) is not a project function.
        if c.qualifier.is_empty() && binds_locally(&files[c.file].2, &defs, c) {
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
            match cands
                .iter()
                .copied()
                .filter(|d| homes.contains(&defs[*d].file))
                .collect::<Vec<_>>()
                .as_slice()
            {
                [one] => Some(*one),
                _ => None,
            }
        };
        // Each link names the rule that made it, so its precision can be measured per rule (FOOTNOTE_CODE_RULES below).
        let typed = c
            .recv
            .as_ref()
            .and_then(|(b, s)| type_of(b, s, c.file))
            .and_then(|(t, file)| method(&t, &c.name, file));
        let target: Option<(usize, &str)> = if let Some(d) = typed {
            Some((d, "receiver_type"))
        } else if !same.is_empty()
            && (c.qualifier.is_empty() || ["self", "this", "Self"].contains(&c.qualifier.as_str()))
        {
            Some((same[0], "same_file"))
        } else if let Some(module) = via {
            if !local(module) {
                // A call into an outside package: counted for the architecture map, never linked.
                let pkg = package_of(files[c.file].1, module);
                docs[c.file].occurrences.push(Occurrence {
                    range: c.range,
                    symbol: format!("treesitter pkg {pkg} . {}.", c.name),
                    roles: 0,
                    enclosing: None,
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
                None => in_module(cands, module),
            };
            match found.as_slice() {
                [one] => Some((*one, "import")),
                _ => None,
            }
        } else if c.member {
            None // a method on some value: its type is unknown here, and std or a dependency may define it
        } else if let Some(one) = if rust {
            // Rust: only a method of that very type. The file rule linked `ByteBuf::default()` (derived) to a neighbour's
            // `impl Default for Phase` (brotli, 18 Nov 2026).
            method(&c.qualifier, &c.name, c.file)
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
                roles: 0,
                enclosing: None,
            });
            rules.push(format!(
                "{}\t{}\t{}\t{}\t{rule}",
                files[c.file].0,
                c.range[0] + 1,
                c.name,
                files[defs[t].file].0
            ));
            resolved += 1;
        }
    }
    Ok((docs, resolved, rules))
}

/// Whether the innermost item around a call declares the called name before the call: `let f =`, `const f`, `var f`,
/// `f := `, `f = `, a parameter `f:` or a closure argument `|f|`. Text patterns, not scopes: a miss only loses a link.
fn binds_locally(src: &[u8], defs: &[Def], c: &Call) -> bool {
    let Some(item) = defs
        .iter()
        .filter(|d| d.file == c.file && d.span[0] <= c.range[0] && c.range[0] <= d.span[2])
        .min_by_key(|d| d.span[2] - d.span[0])
    else {
        return false;
    };
    let text = String::from_utf8_lossy(src);
    let before: String = text
        .lines()
        .skip(item.span[0] as usize)
        .take((c.range[0] - item.span[0]) as usize + 1)
        .collect::<Vec<_>>()
        .join("\n");
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
        "trait_item" => "name",
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
    ["Result", "Option"]
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
            !(e.file_type().is_dir()
                && e.depth() > 0
                && SKIP.contains(&e.file_name().to_string_lossy().as_ref()))
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
