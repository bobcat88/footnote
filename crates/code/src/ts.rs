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
}

/// One call: the called name, its qualifier (`os` in `os.path.join`, `self`, or empty) and where it is.
struct Call {
    name: String,
    qualifier: String,
    /// Called on a value (`obj.f()`), not through a path (`a::f()`) or bare: only `self`/`this` or an imported module may resolve it.
    member: bool,
    file: usize,
    range: [i32; 4],
}

fn range(n: &Node) -> [i32; 4] {
    let (a, b) = (n.start_position(), n.end_position());
    [a.row as i32, a.column as i32, b.row as i32, b.column as i32]
}

/// Parse every supported file under `root` and resolve its calls. `project` names the project's own package in symbols.
pub fn index(root: &Path, project: &str) -> anyhow::Result<(Vec<Document>, usize)> {
    let mut files: Vec<(String, Lang, Vec<u8>)> = Vec::new();
    for rel in candidates(root) {
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
    let mut parser = Parser::new();
    for (i, (_, l, src)) in files.iter().enumerate() {
        if !compiled.contains_key(l) {
            let (lang, tags, imp) = grammar(*l);
            let (Ok(t), Ok(q)) = (Query::new(&lang, tags), Query::new(&lang, imp)) else {
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
                let kind = if matches!(cap, "definition.function" | "definition.method") {
                    "function"
                } else {
                    "type"
                };
                let signature = node
                    .utf8_text(src)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim();
                defs.push(Def {
                    name: text.into(),
                    kind,
                    file: i,
                    range: range(&n),
                    span: range(&node),
                    signature: signature.chars().take(200).collect(),
                });
            } else if node.kind() != "macro_invocation" {
                // A Rust macro (`env!`, `write!`) is not a call to a function of that name.
                let (qualifier, member) = qualifier(&node, &n, src);
                calls.push(Call {
                    name: text.into(),
                    qualifier,
                    member,
                    file: i,
                    range: range(&n),
                });
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
    };
    let by_name: HashMap<&str, Vec<usize>> =
        defs.iter()
            .enumerate()
            .fold(HashMap::new(), |mut m, (i, d)| {
                m.entry(d.name.as_str()).or_default().push(i);
                m
            });
    let dir = |f: usize| files[f].0.rsplit_once('/').map_or("", |x| x.0);
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
            if d.kind == "function" { "()." } else { "#" }
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
    for c in &calls {
        let cands = by_name
            .get(c.name.as_str())
            .map_or(&[][..], |v| v.as_slice());
        let same: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&d| defs[d].file == c.file)
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
        let target: Option<usize> = if !same.is_empty()
            && (c.qualifier.is_empty() || ["self", "this", "Self"].contains(&c.qualifier.as_str()))
        {
            Some(same[0])
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
            match in_module(cands, module).as_slice() {
                [one] => Some(*one),
                _ => None,
            }
        } else if c.member {
            None // a method on some value: its type is unknown here, and std or a dependency may define it
        } else if let Some(one) = owned_by(&c.qualifier) {
            Some(one)
        } else if cands.len() == 1 {
            Some(cands[0])
        } else {
            match cands
                .iter()
                .copied()
                .filter(|&d| dir(defs[d].file) == dir(c.file))
                .collect::<Vec<_>>()
                .as_slice()
            {
                [one] => Some(*one),
                _ => None,
            }
        };
        if let Some(t) = target {
            docs[c.file].occurrences.push(Occurrence {
                range: c.range,
                symbol: sym(&defs[t]),
                roles: 0,
                enclosing: None,
            });
            resolved += 1;
        }
    }
    Ok((docs, resolved))
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

/// The project's files, relative with `/`: what git tracks or would track (so `.gitignore` is honoured), or, outside a git
/// repository or without git, a walk that skips dependency and build folders.
fn candidates(root: &Path) -> Vec<String> {
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
}
