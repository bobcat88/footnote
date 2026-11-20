//! `footnote stack` (#304): which libraries a project uses. Manifests and lockfiles first; when a project has none, the
//! imports in its source (Footnote Code's parser); when that finds nothing, one short question instead of a guess. The
//! result is recorded in Footnote Memory as proof of the selection, and a re-run reports only what changed. Reads files
//! only: no network, and no dependency file is written (#305).
use anyhow::{Result, bail};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
struct Lib {
    /// `cargo`, `npm`, `pypi` or `go`.
    eco: String,
    name: String,
    version: Option<String>,
    /// True when the version is pinned (a lockfile, `==`, `=`, go.mod), false when it is only the declared range.
    exact: bool,
    /// Where the evidence came from: `manifest`, `import` or `user`.
    source: String,
}

type Libs = BTreeMap<(String, String), Lib>;

const SKIP: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "vendor",
    "venv",
    "__pycache__",
];
const MAX_FILE: u64 = 16 << 20;
const QUESTION: &str = "What is this project: which language (Rust, Python, JavaScript or TypeScript, Go) and is it a web app, a command-line tool or a desktop app?";

impl Lib {
    fn new(eco: &str, name: &str, version: Option<String>, exact: bool, source: &str) -> Lib {
        Lib {
            eco: eco.into(),
            name: name.into(),
            version,
            exact,
            source: source.into(),
        }
    }
    fn json(&self) -> Value {
        json!({"name": self.name, "version": self.version, "exact": self.exact, "ecosystem": self.eco, "source": self.source})
    }
}

fn add(libs: &mut Libs, l: Lib) {
    let slot = libs
        .entry((l.eco.clone(), l.name.clone()))
        .or_insert_with(|| l.clone());
    if slot.version.is_none() || (!slot.exact && l.exact) {
        *slot = l;
    }
}

/// `name`'s version: the one lockfile entry when there is exactly one, else the declared requirement (pinned when it is `=x`).
fn pin(
    lock: &HashMap<String, Vec<String>>,
    name: &str,
    spec: Option<&str>,
) -> (Option<String>, bool) {
    if let Some([v]) = lock.get(name).map(Vec::as_slice) {
        return (Some(v.clone()), true);
    }
    let Some(s) = spec.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, false);
    };
    match s.strip_prefix("==").or_else(|| s.strip_prefix('=')) {
        Some(v) if !v.contains(['*', ',', ' ']) => (Some(v.trim().into()), true),
        _ => (Some(s.into()), false),
    }
}

/// `[[package]]` blocks of Cargo.lock, poetry.lock and uv.lock: name -> versions.
fn toml_lock(text: &str, lock: &mut HashMap<String, Vec<String>>, py: bool) {
    let (mut name, mut version) = (None, None);
    let mut flush = |n: &mut Option<String>, v: &mut Option<String>| {
        if let (Some(n), Some(v)) = (n.take(), v.take()) {
            let n = if py { py_name(&n) } else { n };
            let e = lock.entry(n).or_default();
            if !e.contains(&v) {
                e.push(v);
            }
        }
    };
    for line in text.lines() {
        if line.starts_with("[[package]]") {
            flush(&mut name, &mut version);
        } else if let Some(v) = line.strip_prefix("name = ") {
            name = Some(v.trim().trim_matches('"').into());
        } else if let Some(v) = line.strip_prefix("version = ") {
            version = Some(v.trim().trim_matches('"').into());
        }
    }
    flush(&mut name, &mut version);
}

fn quoted(s: &str) -> Option<&str> {
    let a = s.find('"')? + 1;
    Some(&s[a..a + s[a..].find('"')?])
}

/// Dependencies of one Cargo.toml: `[dependencies]`-style sections (any prefix: dev-, build-, workspace., target.*) in the
/// short form, the inline-table form and the `[dependencies.name]` table form. Path-only dependencies are the project's own.
fn cargo_manifest(text: &str, lock: &HashMap<String, Vec<String>>, libs: &mut Libs) {
    let version_re = Regex::new(r#"\bversion\s*=\s*"([^"]*)""#).unwrap();
    let package_re = Regex::new(r#"\bpackage\s*=\s*"([^"]*)""#).unwrap();
    let mut section = String::new();
    // Table form: name, version, package, is a path dependency.
    let mut table: Option<(String, Option<String>, Option<String>, bool)> = None;
    let emit = |name: &str, spec: Option<&str>, libs: &mut Libs| {
        let (version, exact) = pin(lock, name, spec);
        add(libs, Lib::new("cargo", name, version, exact, "manifest"));
    };
    let flush = |t: &mut Option<(String, Option<String>, Option<String>, bool)>,
                 libs: &mut Libs| {
        if let Some((name, ver, pkg, path)) = t.take()
            && !(path && ver.is_none())
        {
            emit(pkg.as_deref().unwrap_or(&name), ver.as_deref(), libs);
        }
    };
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if let Some(h) = line.strip_prefix('[') {
            flush(&mut table, libs);
            section = h.trim_end_matches(']').trim_matches('[').to_string();
            let mut parts = section.rsplitn(2, '.');
            let (last, before) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            if before.ends_with("dependencies") {
                table = Some((last.trim_matches('"').into(), None, None, false));
            }
            continue;
        }
        if let Some(t) = table.as_mut() {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "version" => t.1 = quoted(v).map(String::from),
                    "package" => t.2 = quoted(v).map(String::from),
                    "path" => t.3 = true,
                    _ => {}
                }
            }
            continue;
        }
        let last = section.rsplit('.').next().unwrap_or("");
        if !last.ends_with("dependencies") {
            continue;
        }
        let Some((key, v)) = line.split_once('=') else {
            continue;
        };
        let name = key.trim().split('.').next().unwrap_or("").trim_matches('"');
        if name.is_empty() {
            continue;
        }
        if v.trim_start().starts_with('{') {
            let ver = version_re.captures(v).map(|c| c[1].to_string());
            if v.contains("path") && ver.is_none() {
                continue;
            }
            let pkg = package_re.captures(v).map(|c| c[1].to_string());
            emit(pkg.as_deref().unwrap_or(name), ver.as_deref(), libs);
        } else if let Some(spec) = quoted(v) {
            emit(name, Some(spec), libs);
        } else {
            emit(name, None, libs); // `name.workspace = true`
        }
    }
    flush(&mut table, libs);
}

fn py_name(n: &str) -> String {
    n.to_lowercase().replace(['_', '.'], "-")
}

/// One PEP 508 requirement (`requests[socks]>=2.0 ; python_version<"4"`): name and its version requirement.
fn py_requirement(line: &str, lock: &HashMap<String, Vec<String>>, libs: &mut Libs) {
    let line = line
        .split(';')
        .next()
        .unwrap_or("")
        .split(" #")
        .next()
        .unwrap_or("")
        .trim();
    let end = line
        .find(|c: char| !(c.is_ascii_alphanumeric() || "_.-".contains(c)))
        .unwrap_or(line.len());
    let (name, rest) = line.split_at(end);
    if name.is_empty() || line.contains("://") || line.contains(" @ ") {
        return; // a URL or VCS requirement: no registry name to document
    }
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix('[')
        .and_then(|r| r.split_once(']'))
        .map_or(rest, |x| x.1);
    let name = py_name(name);
    let (version, exact) = pin(lock, &name, Some(rest.trim()));
    add(libs, Lib::new("pypi", &name, version, exact, "manifest"));
}

fn pyproject(text: &str, lock: &HashMap<String, Vec<String>>, libs: &mut Libs) {
    let (mut section, mut in_array) = (String::new(), false);
    let string_re = Regex::new(r#""([^"]+)""#).unwrap();
    let version_re = Regex::new(r#"\bversion\s*=\s*"([^"]*)""#).unwrap();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if in_array {
            for c in string_re.captures_iter(line.split(']').next().unwrap_or("")) {
                py_requirement(&c[1], lock, libs);
            }
            in_array = !line.contains(']');
        } else if line.starts_with('[') {
            section = line.trim_matches(['[', ']']).to_string();
        } else if section == "project"
            && let Some(rest) = line
                .strip_prefix("dependencies")
                .and_then(|r| r.trim_start().strip_prefix('='))
        {
            for c in string_re.captures_iter(rest.split(']').next().unwrap_or("")) {
                py_requirement(&c[1], lock, libs);
            }
            in_array = !rest.contains(']');
        } else if section.starts_with("tool.poetry")
            && section.ends_with("dependencies")
            && let Some((k, v)) = line.split_once('=')
            && k.trim() != "python"
        {
            let name = py_name(k.trim().trim_matches('"'));
            let spec = if v.trim_start().starts_with('{') {
                version_re.captures(v).map(|c| c[1].to_string())
            } else {
                quoted(v).map(String::from)
            };
            let (version, exact) = pin(lock, &name, spec.as_deref());
            add(libs, Lib::new("pypi", &name, version, exact, "manifest"));
        }
    }
}

fn go_mod(text: &str, libs: &mut Libs) {
    let mut block = false;
    for line in text.lines().map(str::trim) {
        let spec = if block {
            if line.starts_with(')') {
                block = false;
                continue;
            }
            line
        } else if let Some(r) = line.strip_prefix("require") {
            let r = r.trim();
            if r.starts_with('(') {
                block = true;
                continue;
            }
            r
        } else {
            continue;
        };
        if spec.contains("// indirect") {
            continue;
        }
        let mut w = spec.split_whitespace();
        if let (Some(name), Some(v)) = (w.next(), w.next()) {
            add(libs, Lib::new("go", name, Some(v.into()), true, "manifest"));
        }
    }
}

fn package_json(text: &str, lock: Option<&Value>, libs: &mut Libs) {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return;
    };
    for kind in ["dependencies", "devDependencies", "optionalDependencies"] {
        for (name, spec) in v[kind].as_object().into_iter().flatten() {
            let spec = spec.as_str().unwrap_or("");
            if ["file:", "link:", "workspace:"]
                .iter()
                .any(|p| spec.starts_with(p))
            {
                continue; // the project's own packages
            }
            let locked = lock.and_then(|l| {
                l["packages"][format!("node_modules/{name}")]["version"]
                    .as_str()
                    .or_else(|| l["dependencies"][name]["version"].as_str())
            });
            let (version, exact) = match locked {
                Some(v) => (Some(v.to_string()), true),
                None => (Some(spec.to_string()).filter(|s| !s.is_empty()), false),
            };
            add(libs, Lib::new("npm", name, version, exact, "manifest"));
        }
    }
}

/// The Python distribution that provides an import name, for the names that differ (the rest are the module name, lowercased).
fn pypi_name(module: &str) -> String {
    const KNOWN: &[(&str, &str)] = &[
        ("yaml", "pyyaml"),
        ("PIL", "pillow"),
        ("cv2", "opencv-python"),
        ("sklearn", "scikit-learn"),
        ("skimage", "scikit-image"),
        ("bs4", "beautifulsoup4"),
        ("dotenv", "python-dotenv"),
        ("dateutil", "python-dateutil"),
        ("jwt", "pyjwt"),
        ("git", "gitpython"),
        ("attr", "attrs"),
        ("serial", "pyserial"),
        ("OpenSSL", "pyopenssl"),
        ("Crypto", "pycryptodome"),
        ("docx", "python-docx"),
        ("magic", "python-magic"),
    ];
    KNOWN
        .iter()
        .find(|k| k.0 == module)
        .map_or_else(|| py_name(module), |k| k.1.into())
}

fn read(path: &Path) -> Option<String> {
    std::fs::metadata(path)
        .ok()
        .filter(|m| m.len() <= MAX_FILE)?;
    std::fs::read_to_string(path).ok()
}

/// Every library the manifests and lockfiles under `root` declare (depth 5; dependency and build folders skipped).
fn from_manifests(root: &Path) -> Libs {
    let walk = walkdir::WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            e.depth() == 0
                || !(e.file_type().is_dir() && (n.starts_with('.') || SKIP.contains(&n.as_ref())))
        });
    let (mut cargo_lock, mut py_lock) = (HashMap::new(), HashMap::new());
    let mut npm_lock: Option<Value> = None;
    let mut manifests = Vec::new();
    for e in walk.flatten().filter(|e| e.file_type().is_file()) {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(text) = ["Cargo.lock", "poetry.lock", "uv.lock", "package-lock.json"]
            .contains(&name.as_str())
            .then(|| read(e.path()))
            .flatten()
        else {
            if ["Cargo.toml", "package.json", "pyproject.toml", "go.mod"].contains(&name.as_str())
                || (name.starts_with("requirements") && name.ends_with(".txt"))
            {
                manifests.push((name, e.path().to_path_buf()));
            }
            continue;
        };
        match name.as_str() {
            "Cargo.lock" => toml_lock(&text, &mut cargo_lock, false),
            "package-lock.json" if npm_lock.is_none() => {
                npm_lock = serde_json::from_str(&text).ok()
            }
            "package-lock.json" => {}
            _ => toml_lock(&text, &mut py_lock, true),
        }
    }
    let mut libs = Libs::new();
    for (name, path) in manifests {
        let Some(text) = read(&path) else { continue };
        match name.as_str() {
            "Cargo.toml" => cargo_manifest(&text, &cargo_lock, &mut libs),
            "package.json" => package_json(&text, npm_lock.as_ref(), &mut libs),
            "pyproject.toml" => pyproject(&text, &py_lock, &mut libs),
            "go.mod" => go_mod(&text, &mut libs),
            _ => {
                for l in text.lines().map(str::trim) {
                    if !l.is_empty() && !l.starts_with(['#', '-']) {
                        py_requirement(l, &py_lock, &mut libs);
                    }
                }
            }
        }
    }
    libs
}

fn from_imports(root: &Path) -> Libs {
    let mut libs = Libs::new();
    for (lang, pkg) in footnote_code::imports(root) {
        let (eco, name) = match lang {
            "rust" => ("cargo", pkg),
            "python" => ("pypi", pypi_name(&pkg)),
            "go" => ("go", pkg),
            _ => ("npm", pkg),
        };
        add(&mut libs, Lib::new(eco, &name, None, false, "import"));
    }
    libs
}

/// The libraries as they stand: manifests, else imports, plus what the user told (`--library eco:name[@version]`).
fn detect(root: &Path, user: &[String]) -> Result<Libs> {
    let mut libs = from_manifests(root);
    if libs.is_empty() {
        libs = from_imports(root);
    }
    for u in user {
        let Some((eco, rest)) = u
            .split_once(':')
            .filter(|(e, r)| ["cargo", "npm", "pypi", "go"].contains(e) && !r.is_empty())
        else {
            bail!(
                "--library takes <ecosystem>:<name>[@version] with ecosystem cargo, npm, pypi or go, not {u:?}"
            );
        };
        // A scoped npm name starts with `@`; the version follows the last one.
        let (name, version) = match rest.rfind('@').filter(|&i| i > 0) {
            Some(i) => (&rest[..i], Some(rest[i + 1..].to_string())),
            None => (rest, None),
        };
        let exact = version.is_some();
        libs.insert(
            (eco.into(), name.into()),
            Lib::new(eco, name, version, exact, "user"),
        );
    }
    Ok(libs)
}

fn describe(l: &Lib) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        l.eco,
        l.name,
        l.version.as_deref().unwrap_or("-"),
        if l.exact { "exact" } else { "declared" },
        l.source
    )
}

fn parse_line(line: &str) -> Option<Lib> {
    let p: Vec<&str> = line.split(" | ").collect();
    let [eco, name, version, exact, source] = p[..] else {
        return None;
    };
    Some(Lib {
        eco: eco.into(),
        name: name.into(),
        version: (version != "-").then(|| version.into()),
        exact: exact == "exact",
        source: source.into(),
    })
}

fn diff(before: &Libs, after: &Libs) -> Value {
    let key = |l: &Lib| (l.eco.clone(), l.name.clone());
    let added: Vec<Value> = after
        .values()
        .filter(|l| !before.contains_key(&key(l)))
        .map(Lib::json)
        .collect();
    let removed: Vec<Value> = before
        .values()
        .filter(|l| !after.contains_key(&key(l)))
        .map(Lib::json)
        .collect();
    let changed: Vec<Value> = after
        .values()
        .filter_map(|l| {
            let b = before.get(&key(l)).filter(|b| b.version != l.version)?;
            Some(json!({"name": l.name, "ecosystem": l.eco, "from": b.version, "to": l.version, "exact": l.exact}))
        })
        .collect();
    json!({"added": added, "removed": removed, "changed": changed})
}

/// The recorded stack of this folder: the live memory ids and the libraries their text holds.
#[cfg(feature = "memory")]
fn previous(c: &rusqlite::Connection, scope: &str, root: &str) -> Result<(Vec<i64>, Libs)> {
    let head = format!("Detected stack of {root} (part ");
    let mut q = c.prepare(
        "SELECT s.id, ch.text FROM memories m JOIN sources s ON s.id=m.source_id JOIN chunks ch ON ch.source_id=s.id \
         WHERE m.scope=?1 AND s.status='indexed' AND substr(ch.text,1,length(?2))=?2 ORDER BY s.id",
    )?;
    let rows = q.query_map(rusqlite::params![scope, head], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let (mut ids, mut libs) = (vec![], Libs::new());
    for row in rows {
        let (id, text) = row?;
        ids.push(id);
        for l in text.lines().skip(1).filter_map(parse_line) {
            libs.insert((l.eco.clone(), l.name.clone()), l);
        }
    }
    Ok((ids, libs))
}

/// Memory texts of at most 2000 characters each: the header names the folder, one line per library.
#[cfg(feature = "memory")]
fn memory_items(root: &str, scope: &str, libs: &Libs, today: &str) -> Vec<Value> {
    let lines: Vec<String> = libs.values().map(describe).collect();
    let mut parts: Vec<String> = vec![];
    for line in &lines {
        match parts.last_mut() {
            Some(p) if p.len() + line.len() < 1800 => {
                p.push('\n');
                p.push_str(line);
            }
            _ => parts.push(line.clone()),
        }
    }
    let n = parts.len();
    parts
        .iter()
        .enumerate()
        .map(|(i, body)| {
            json!({"text": format!("Detected stack of {root} (part {}/{n})\n{body}", i + 1), "kind": "fact", "scope": scope, "observed_at": today})
        })
        .collect()
}

/// `footnote stack`. `record`: write the result to Footnote Memory (the CLI); the MCP tool only reads.
pub fn run(root: &Path, user: &[String], record: bool) -> Result<Value> {
    let root = std::path::absolute(root)?;
    if !root.is_dir() {
        bail!("{} is not a folder", root.display());
    }
    let libs = detect(&root, user)?;
    let root_s = root.to_string_lossy().to_string();
    if libs.is_empty() {
        return Ok(
            json!({"root": root_s, "status": "needs_answer", "question": QUESTION, "libraries": [],
            "next": "answer with `footnote stack <path> --library <ecosystem>:<name>[@version]` for what the project uses"}),
        );
    }
    #[cfg(feature = "memory")]
    {
        let state = crate::state::state_dir()?;
        let db = crate::status::memory_db(&state)?;
        let scope = crate::hook::project_of(&root_s);
        let today = time::OffsetDateTime::now_utc().date().to_string();
        let (ids, before) = if db.exists() {
            previous(&crate::app::open(&db, false)?, &scope, &root_s)?
        } else {
            (vec![], Libs::new())
        };
        let mut out = json!({"root": root_s, "project": scope, "count": libs.len()});
        let first = ids.is_empty();
        let changes = diff(&before, &libs);
        let same = changes
            .as_object()
            .is_some_and(|c| c.values().all(|v| v.as_array().is_some_and(Vec::is_empty)));
        if first {
            out["status"] = json!("detected");
            out["libraries"] = libs.values().map(Lib::json).collect();
        } else {
            out["status"] = json!(if same { "unchanged" } else { "changed" });
            out["changes"] = changes;
        }
        out["recorded"] = json!(false);
        if record && (first || !same) {
            std::fs::create_dir_all(&state)?;
            let mut c = crate::app::open(&db, true)?;
            for id in ids {
                crate::memory::forget(&mut c, id)?;
            }
            crate::memory::remember(
                &mut c,
                &json!(memory_items(&root_s, &scope, &libs, &today)),
                &[],
                "cli",
            )?;
            out["recorded"] = json!(true);
        }
        Ok(out)
    }
    #[cfg(not(feature = "memory"))]
    {
        let _ = record;
        Ok(
            json!({"root": root_s, "status": "detected", "count": libs.len(), "recorded": false,
            "libraries": libs.values().map(Lib::json).collect::<Vec<_>>()}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_forms_and_lock_versions() {
        let lock: HashMap<String, Vec<String>> =
            [("serde".to_string(), vec!["1.0.229".to_string()])].into();
        let mut libs = Libs::new();
        cargo_manifest(
            "[workspace.dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nlocal = { path = \"crates/x\" }\n\
             [dependencies]\nanyhow = \"1\"\nzip = { package = \"zip-rs\", version = \"=2.1.0\" }\nclap.workspace = true\n\
             [dependencies.url]\nversion = \"2\"\n",
            &lock,
            &mut libs,
        );
        let v = |n: &str| {
            let l = &libs[&("cargo".to_string(), n.to_string())];
            (l.version.clone(), l.exact)
        };
        assert_eq!(v("serde"), (Some("1.0.229".into()), true));
        assert_eq!(v("anyhow"), (Some("1".into()), false));
        assert_eq!(v("zip-rs"), (Some("2.1.0".into()), true));
        assert_eq!(v("clap"), (None, false));
        assert_eq!(v("url"), (Some("2".into()), false));
        assert!(!libs.contains_key(&("cargo".to_string(), "local".to_string())));
    }

    #[test]
    fn python_and_go_formats() {
        let mut libs = Libs::new();
        let none = HashMap::new();
        py_requirement(
            "Requests[socks]==2.31.0 ; python_version > '3'",
            &none,
            &mut libs,
        );
        py_requirement("flask>=2.0", &none, &mut libs);
        pyproject(
            "[project]\ndependencies = [\n  \"httpx>=0.27\",\n  \"rich\",\n]\n",
            &none,
            &mut libs,
        );
        go_mod(
            "module x\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n\tgolang.org/x/sys v0.1.0 // indirect\n)\n",
            &mut libs,
        );
        let got: Vec<String> = libs.values().map(describe).collect();
        assert_eq!(
            got,
            [
                "go | github.com/gin-gonic/gin | v1.9.1 | exact | manifest",
                "pypi | flask | >=2.0 | declared | manifest",
                "pypi | httpx | >=0.27 | declared | manifest",
                "pypi | requests | 2.31.0 | exact | manifest",
                "pypi | rich | - | declared | manifest",
            ]
        );
    }

    #[test]
    fn recorded_lines_parse_back() {
        let l = Lib::new("npm", "@scope/pkg", Some("1.2.3".into()), true, "manifest");
        assert_eq!(parse_line(&describe(&l)), Some(l));
    }
}
