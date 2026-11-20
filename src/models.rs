//! Embedding model manager: list, install, use, remove, recommend (P1 item 9).
//!
//! `models install` is the only command that reaches the network: it fetches public model files pinned to a Hugging Face commit
//! and checks them against pinned SHA-256 hashes. The manifest it writes is byte-identical to the one the former Python script
//! wrote, because a model's identity is the hash of its manifest (vectors computed before stay valid).
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

struct Recipe {
    name: &'static str,
    repo: &'static str,
    revision: &'static str,
    /// The model file in the repository: `.onnx` (with `tokenizer.json`), or `.gguf` (llama.cpp, tokenizer inside).
    file: &'static str,
    /// What `install` downloads, in bytes (model file plus tokenizer at the pinned revision, from the Hugging Face API).
    download_bytes: u64,
    query_prefix: &'static str,
    passage_prefix: &'static str,
    max_length: u32,
    pad_token: &'static str,
    quantization: &'static str,
    /// Pinned (file, sha256) of every downloaded file; a download that does not match is removed.
    sha256: &'static [(&'static str, &'static str)],
    role: &'static str,
    /// Written into the manifest and shown on install when it restricts use; empty for the permissive ONNX models.
    license: &'static str,
}

impl Recipe {
    fn gguf(&self) -> bool {
        self.file.ends_with(".gguf")
    }
}

const RECIPES: &[Recipe] = &[
    Recipe {
        name: "bekko-a8m",
        download_bytes: 164462521,
        repo: "hotchpotch/bekko-embedding-v1-a8m",
        revision: "c721113d59a1d91b447450324f51c4b3332c924a",
        file: "onnx/model.onnx",
        query_prefix: "",
        passage_prefix: "",
        max_length: 512,
        pad_token: "<pad>",
        quantization: "none (fp32)",
        sha256: &[
            (
                "model.onnx",
                "96d8cc6199e96357b21b2fb12f6d7ffd2d4abc7b182fe94b5468fbd6dc819af7",
            ),
            (
                "tokenizer.json",
                "8bd47075711f75a143d1b78e01a41cc65c1c591b00d3cfeffc23db07adce1392",
            ),
        ],
        role: "default pair, multilingual half",
        license: "MIT",
    },
    Recipe {
        name: "gte-small",
        download_bytes: 34726087,
        repo: "Xenova/gte-small",
        revision: "5927d1727bb12db490052a1b33265ad78058de08",
        file: "onnx/model_quantized.onnx",
        query_prefix: "",
        passage_prefix: "",
        max_length: 512,
        pad_token: "[PAD]",
        quantization: "onnx-dynamic-int8",
        sha256: &[
            (
                "model.onnx",
                "18dec105109b6004369799ca4761fb8fb413c64172c02147bcfac186b5c5f6cb",
            ),
            (
                "tokenizer.json",
                "da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0",
            ),
        ],
        role: "default pair, English half",
        license: "MIT",
    },
    Recipe {
        name: "e5-small",
        download_bytes: 135390915,
        repo: "Xenova/multilingual-e5-small",
        revision: "761b726dd34fb83930e26aab4e9ac3899aa1fa78",
        file: "onnx/model_quantized.onnx",
        query_prefix: "query: ",
        passage_prefix: "passage: ",
        max_length: 512,
        pad_token: "<pad>",
        quantization: "onnx-dynamic-int8",
        sha256: &[
            (
                "model.onnx",
                "f80102d3f2a1229f387d3c81909990d8945513e347b0eab049f7de3c6f98c193",
            ),
            (
                "tokenizer.json",
                "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39",
            ),
        ],
        role: "earlier model, tests and comparison",
        license: "MIT",
    },
    Recipe {
        name: "minilm",
        download_bytes: 135391039,
        repo: "Xenova/paraphrase-multilingual-MiniLM-L12-v2",
        revision: "2c4055b12046f11709e9df2c122e59ffbdc2f900",
        file: "onnx/model_quantized.onnx",
        query_prefix: "",
        passage_prefix: "",
        max_length: 128,
        pad_token: "<pad>",
        quantization: "onnx-dynamic-int8",
        sha256: &[
            (
                "model.onnx",
                "66fc00f5f29afcaff34092e1bdd20008ca3918265a82fb9695a551e510cc4ebc",
            ),
            (
                "tokenizer.json",
                "b60b6b43406a48bf3638526314f3d232d97058bc93472ff2de930d43686fa441",
            ),
        ],
        role: "earlier model, tests and comparison",
        license: "Apache-2.0",
    },
    Recipe {
        name: "jina-v5-nano",
        download_bytes: 232883776,
        repo: "jinaai/jina-embeddings-v5-text-nano-retrieval-GGUF",
        revision: "59cfaceeeb7d738c404659435af4c0da74d06c96",
        file: "v5-nano-retrieval-Q8_0.gguf",
        query_prefix: "Query: ",
        passage_prefix: "Document: ",
        max_length: 512,
        pad_token: "",
        quantization: "Q8_0",
        sha256: &[(
            "model.gguf",
            "86b6e6279e9b9e71389f02a082764a2ac2b15a50e37482c26f98d69092f12442",
        )],
        role: "French or mixed corpora, opt-in (llama build)",
        license: "CC-BY-NC-4.0 (no commercial use)",
    },
];
const DEFAULT_PAIR: [&str; 2] = ["bekko-a8m", "gte-small"];

fn recipe(name: &str) -> Result<&'static Recipe> {
    RECIPES
        .iter()
        .find(|r| r.name == name)
        .with_context(|| format!("unknown model `{name}`; `footnote models list` shows them"))
}

pub fn models_dir(state: &Path) -> PathBuf {
    state.join("models")
}

/// Models the memory hook uses (memory.json `models`), as written there.
pub(crate) fn in_use(state: &Path) -> Result<Vec<String>> {
    let s = footnote_core::config::product_settings(state, "memory")?;
    Ok(s["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str().map(|p| p.replace('\\', "/")))
        .collect())
}

fn same_model(path: &str, dir: &Path) -> bool {
    let dir = dir.to_string_lossy().replace('\\', "/");
    path.trim_end_matches('/') == dir.trim_end_matches('/')
}

pub fn list(state: &Path) -> Result<Value> {
    let used = in_use(state)?;
    let rows: Vec<Value> = RECIPES
        .iter()
        .map(|r| {
            let dir = models_dir(state).join(r.name);
            let installed = dir.join("manifest.json").exists();
            let bytes: u64 = ["model.onnx", "tokenizer.json", "model.gguf"]
                .iter()
                .filter_map(|f| fs::metadata(dir.join(f)).ok())
                .map(|m| m.len())
                .sum();
            json!({"name": r.name, "role": r.role, "installed": installed, "path": installed.then(|| dir.clone()),
                "bytes": installed.then_some(bytes), "download_bytes": r.download_bytes, "used_by_memory": used.iter().any(|u| same_model(u, &dir)),
                "source": format!("{}@{}", r.repo, &r.revision[..12]),
                "license": (!r.license.is_empty()).then_some(r.license)})
        })
        .collect();
    // Models the hook uses from elsewhere (for example a development checkout) are reported too.
    let elsewhere: Vec<&String> = used
        .iter()
        .filter(|u| {
            !RECIPES
                .iter()
                .any(|r| same_model(u, &models_dir(state).join(r.name)))
        })
        .collect();
    Ok(
        json!({"models": rows, "memory_uses_from_elsewhere": elsewhere, "folder": models_dir(state)}),
    )
}

fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier) // system roots, so company TLS proxies work (as in crawl.rs)
                .build(),
        )
        .build()
        .into()
}

/// Stream `url` into `to`, reporting bytes against the size the server announces.
fn download(url: &str, to: &Path, job: &mut footnote_core::jobs::Job) -> Result<()> {
    use std::io::Write;
    let response = agent()
        .get(url)
        .call()
        .with_context(|| format!("download {url}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let mut body = response.into_body();
    let mut reader = body.as_reader();
    let mut out = fs::File::create(to)?;
    let (mut buf, mut done) = (vec![0u8; 1 << 16], 0u64);
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("download {url}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        done += n as u64;
        job.progress(done, total);
    }
    job.progress(done, total.or(Some(done)));
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

/// The manifest exactly as the former Python script wrote it (`json.dumps(indent=2)` plus a newline, in text mode, so CRLF on
/// Windows): a model's identity is the hash of these bytes.
fn manifest_text(r: &Recipe, files: &[(&str, String, u64, String)]) -> String {
    let s = |v: &str| serde_json::to_string(v).expect("string");
    let entries: Vec<String> = files
        .iter()
        .map(|(local, sha, bytes, url)| {
            format!(
                "    {}: {{\n      \"sha256\": {},\n      \"bytes\": {bytes},\n      \"url\": {}\n    }}",
                s(local),
                s(sha),
                s(url)
            )
        })
        .collect();
    if r.gguf() {
        // Byte-identical to the manifest the benchmark lab wrote for test 6 (LF on every platform), so vectors stay valid.
        // shortcut: pooling and width are Jina v5 nano's, generalise when a second GGUF model is offered.
        return format!(
            "{{
  \"repository\": {},
  \"revision\": {},
  \"license\": {},
  \"runtime\": \"llama.cpp\",
  \"files\": {{
{}
  }},
  \"pooling\": \"last\",
  \"query_prefix\": {},
  \"passage_prefix\": {},
  \"max_length\": {},
  \"dimension\": 768,
  \"quantization\": {}
}}
",
            s(r.repo),
            s(r.revision),
            s(r.license),
            entries.join(
                ",
"
            ),
            s(r.query_prefix),
            s(r.passage_prefix),
            r.max_length,
            s(r.quantization)
        );
    }
    let text = format!(
        "{{\n  \"repository\": {},\n  \"revision\": {},\n  \"files\": {{\n{}\n  }},\n  \"pooling\": \"attention-mask-mean-l2\",\n  \"query_prefix\": {},\n  \"passage_prefix\": {},\n  \"max_length\": {},\n  \"pad_token\": {},\n  \"dimension\": 384,\n  \"quantization\": {}\n}}\n",
        s(r.repo),
        s(r.revision),
        entries.join(",\n"),
        s(r.query_prefix),
        s(r.passage_prefix),
        r.max_length,
        s(r.pad_token),
        s(r.quantization)
    );
    if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text
    }
}

pub fn install(state: &Path, names: &[String]) -> Result<Value> {
    let mut done = Vec::new();
    for name in names {
        let r = recipe(name)?;
        if r.gguf() && !cfg!(feature = "llama") {
            bail!("`{name}` runs through llama.cpp, which this build of Footnote does not include");
        }
        let dir = models_dir(state).join(r.name);
        ignore_models(state)?;
        let mut files = Vec::new();
        let wanted: &[(&str, &str)] = if r.gguf() {
            &[(r.file, "model.gguf")]
        } else {
            &[("tokenizer.json", "tokenizer.json"), (r.file, "model.onnx")]
        };
        for &(remote, local) in wanted {
            let path = dir.join(local);
            let url = format!(
                "https://huggingface.co/{}/resolve/{}/{remote}",
                r.repo, r.revision
            );
            let pinned = r.sha256.iter().find(|(f, _)| *f == local).map(|p| p.1);
            let digest = fetch(state, r.name, &path, &url, pinned)?;
            files.push((local, digest, fs::metadata(&path)?.len(), url));
        }
        fs::write(dir.join("manifest.json"), manifest_text(r, &files))?;
        done.push(json!({"name": r.name, "path": dir, "license": (!r.license.is_empty()).then_some(r.license)}));
    }
    Ok(json!({"installed": done}))
}

/// Hundreds of MB that must never reach a repository, even if the state folder ends up inside a synced git folder.
fn ignore_models(state: &Path) -> Result<()> {
    let ignore = models_dir(state).join(".gitignore");
    if !ignore.exists() {
        fs::create_dir_all(models_dir(state))?;
        fs::write(
            &ignore,
            "# Downloaded models: never commit them.
*
",
        )?;
    }
    Ok(())
}

/// Download `url` to `path` unless present, then check its sha256 against `pinned` (a mismatching file is removed).
fn fetch(state: &Path, name: &str, path: &Path, url: &str, pinned: Option<&str>) -> Result<String> {
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !path.exists() {
        fs::create_dir_all(path.parent().context("model file path")?)?;
        let mut partial = path.as_os_str().to_owned();
        partial.push(".partial");
        let partial = PathBuf::from(partial);
        let mut job = footnote_core::jobs::Job::start(
            state,
            "models.install",
            &format!("{name} {file}"),
            "bytes",
        )?;
        let done = download(url, &partial, &mut job);
        job.finish(&done);
        done?;
        fs::rename(&partial, path)?;
    }
    let digest = sha256_file(path)?;
    if let Some(pinned) = pinned
        && digest != pinned
    {
        fs::remove_file(path)?;
        bail!("{name}/{file}: sha256 {digest} does not match the pinned {pinned}; file removed");
    }
    Ok(digest)
}

/// Point the memory hook at these installed models. Vectors for the new models are computed at the next session start.
pub fn use_models(state: &Path, names: &[String]) -> Result<Value> {
    let mut paths = Vec::new();
    for name in names {
        recipe(name)?;
        let dir = models_dir(state).join(name);
        if !dir.join("manifest.json").exists() {
            bail!("`{name}` is not installed; run `footnote models install {name}` first");
        }
        paths.push(dir.to_string_lossy().replace('\\', "/"));
    }
    if paths.is_empty() || paths.len() > 3 {
        bail!("give one to three models");
    }
    footnote_core::config::set_value(state, "memory", "models", json!(paths))?;
    Ok(
        json!({"memory_models": paths, "note": "vectors these models do not have yet are computed in the background at the next session start (none when only the folder changed)"}),
    )
}

pub fn remove(state: &Path, name: &str) -> Result<Value> {
    recipe(name)?;
    let dir = models_dir(state).join(name);
    if in_use(state)?.iter().any(|u| same_model(u, &dir)) {
        bail!("`{name}` is used by Footnote Memory; switch with `footnote models use` first");
    }
    if !dir.exists() {
        return Ok(json!({"removed": false, "name": name}));
    }
    footnote_core::no_links(&dir)?;
    fs::remove_dir_all(&dir)?;
    Ok(json!({"removed": true, "name": name}))
}

/// French share of a corpus sample from function words: an estimate, not a language detector.
fn corpus_language(db: &Path) -> Result<Value> {
    let c = crate::app::open(db, false)?;
    let mut s = c.prepare("SELECT text FROM chunks ORDER BY id LIMIT 400")?;
    let texts: Vec<String> = s
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let (mut fr, mut en) = (0usize, 0usize);
    for t in &texts {
        for w in t
            .split(|ch: char| !ch.is_alphabetic())
            .map(str::to_lowercase)
        {
            match w.as_str() {
                "le" | "la" | "les" | "des" | "est" | "une" | "pour" | "que" | "dans" | "avec"
                | "sur" | "pas" => fr += 1,
                "the" | "and" | "is" | "of" | "to" | "for" | "that" | "with" | "on" | "not"
                | "are" | "this" => en += 1,
                _ => {}
            }
        }
    }
    let share = if fr + en == 0 {
        0.0
    } else {
        fr as f64 / (fr + en) as f64
    };
    let label = if fr + en < 50 {
        "too little text"
    } else if share >= 0.8 {
        "French"
    } else if share > 0.2 {
        "mixed French and English"
    } else {
        "English"
    };
    Ok(
        json!({"estimate": label, "french_share": (share * 100.0).round() / 100.0, "chunks_sampled": texts.len()}),
    )
}

/// Recommendation from Footnote's own measured results only; where nothing was measured, it says so.
pub fn recommend(db: Option<&Path>) -> Result<Value> {
    let hardware = crate::hardware::detect();
    let language = db.map(corpus_language).transpose()?;
    let french = language.as_ref().is_some_and(|l| {
        matches!(
            l["estimate"].as_str(),
            Some("French" | "mixed French and English")
        )
    });
    let mut reasons = vec![json!(
        "The default pair (bekko-a8m + gte-small) is the shipped default, chosen with benchmark protocol v1 (bench/experiments/RESULTS.md)."
    )];
    if french {
        reasons.push(json!(
            "French or mixed corpus: Jina v5 nano scored +0.08 to +0.10 nDCG@10 over the pair on held-out Alloprof and mix-fr-en (bench/experiments/LLAMACPP-INPROCESS.md). On an idle machine it adds about 6 ms at p50 and 26 ms at p95 per search; the database is about 2.5 times larger. Licence CC-BY-NC-4.0: no commercial use. Install with `footnote models install jina-v5-nano` (builds that include llama.cpp)."
        ));
    }
    if hardware["gpus"].as_array().is_some_and(|g| !g.is_empty()) {
        reasons.push(json!(
            "GPU found but not used for search: on the Radeon RX 6800, single queries through DirectML were 10 to 34 times slower than the CPU; only batch indexing with fp32 bekko-a8m gained (3.2 times) (bench/experiments/GPU-ENCODING.md)."
        ));
    }
    Ok(json!({
        "hardware": hardware,
        "corpus_language": language,
        "recommended": DEFAULT_PAIR,
        "consider": if french { json!(["jina-v5-nano"]) } else { json!([]) },
        "reasons": reasons,
    }))
}

#[cfg(test)]
mod tests {
    /// Same bytes as the manifest the lab wrote for test 6, so a model's identity (the manifest hash) does not change.
    #[test]
    fn nano_manifest_matches_the_benchmarked_one() {
        let r = super::recipe("jina-v5-nano").unwrap();
        let url = "https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval-GGUF/resolve/59cfaceeeb7d738c404659435af4c0da74d06c96/v5-nano-retrieval-Q8_0.gguf";
        let files = [(
            "model.gguf",
            r.sha256[0].1.to_string(),
            232883776,
            url.to_string(),
        )];
        let expected = r#"{
  "repository": "jinaai/jina-embeddings-v5-text-nano-retrieval-GGUF",
  "revision": "59cfaceeeb7d738c404659435af4c0da74d06c96",
  "license": "CC-BY-NC-4.0 (no commercial use)",
  "runtime": "llama.cpp",
  "files": {
    "model.gguf": {
      "sha256": "86b6e6279e9b9e71389f02a082764a2ac2b15a50e37482c26f98d69092f12442",
      "bytes": 232883776,
      "url": "https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval-GGUF/resolve/59cfaceeeb7d738c404659435af4c0da74d06c96/v5-nano-retrieval-Q8_0.gguf"
    }
  },
  "pooling": "last",
  "query_prefix": "Query: ",
  "passage_prefix": "Document: ",
  "max_length": 512,
  "dimension": 768,
  "quantization": "Q8_0"
}
"#;
        assert_eq!(
            super::manifest_text(r, &files),
            expected.replace("\r\n", "\n")
        );
    }
}
