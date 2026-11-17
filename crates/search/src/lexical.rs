//! Lexical ranking with Tantivy (BM25, English stemmer, accent folding).
//!
//! The index is *derived data*: SQLite stays the system of record and decides what exists and what is current. The Tantivy folder
//! next to the database only answers "which chunk ids match these words best". Chunk ids come from AUTOINCREMENT, so the index is
//! append-only: new chunks are added when `sqlite_sequence` moved, and chunks that were deleted or whose source is no longer
//! `indexed` are dropped by the SQL join that follows. Any failure falls back to FTS5 (`app::search`), never to a wrong answer.
//!
//! Bench evidence (bench/DECISION.md): same precision as FTS5, 0.38 ms vs 43 ms at 100k documents, 3 ms vs 423 ms at 1M.
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tantivy::{
    Index, IndexReader, IndexWriter, TantivyDocument, Term,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, Occur, Query, TermQuery},
    schema::{
        FAST, Field, IndexRecordOption, STORED, Schema, TextFieldIndexing, TextOptions, Value,
    },
    tokenizer::{
        AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer,
        TextAnalyzer,
    },
};

const ANALYZER: &str = "footnote_en";
const META: &str = "footnote.json";
const WRITER_HEAP: usize = 256_000_000;

struct Open {
    dir: PathBuf,
    built_seq: i64,
    cipher_salt: Option<String>,
    index: Index,
    reader: IndexReader,
    id: Field,
    body: Field,
}
static OPEN: OnceLock<Mutex<Option<Open>>> = OnceLock::new();

fn analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .filter(Stemmer::new(Language::English))
        .build()
}
fn schema() -> (Schema, Field, Field) {
    let mut b = Schema::builder();
    let id = b.add_u64_field("id", STORED | FAST);
    let body = b.add_text_field(
        "body",
        TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(ANALYZER)
                .set_index_option(IndexRecordOption::WithFreqs),
        ),
    );
    (b.build(), id, body)
}
/// Folder of the derived index: `<database file>.tantivy`. None for in-memory databases.
fn dir_of(c: &Connection) -> Option<PathBuf> {
    let p = c.path().filter(|p| !p.is_empty())?;
    Some(PathBuf::from(format!("{p}.tantivy")))
}
// Fail closed if SQLCipher cannot report its state: never select an on-disk index by guessing.
fn cipher_salt(c: &Connection) -> Result<Option<String>> {
    #[cfg(feature = "encryption")]
    {
        let active: String = c.pragma_query_value(None, "cipher_status", |r| r.get(0))?;
        match active.as_str() {
            "0" => Ok(None),
            "1" => Ok(Some(
                c.pragma_query_value(None, "cipher_salt", |r| r.get(0))?,
            )),
            _ => anyhow::bail!("cannot determine database encryption state"),
        }
    }
    #[cfg(not(feature = "encryption"))]
    {
        let _ = c;
        Ok(None)
    }
}
fn sequence(c: &Connection) -> Result<i64> {
    Ok(c.query_row(
        "SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name='chunks'),0)",
        [],
        |r| r.get(0),
    )?)
}
fn read_built(dir: &Path) -> Option<i64> {
    let v: serde_json::Value = serde_json::from_slice(&fs::read(dir.join(META)).ok()?).ok()?;
    v["built_seq"].as_i64()
}
fn write_built(dir: &Path, seq: i64) -> Result<()> {
    fs::write(
        dir.join(META),
        serde_json::json!({"built_seq":seq,"analyzer":ANALYZER,"tantivy":"0.27"}).to_string(),
    )?;
    Ok(())
}
fn add_chunks(
    c: &Connection,
    writer: &mut IndexWriter,
    id: Field,
    body: Field,
    after: i64,
) -> Result<()> {
    let mut s = c.prepare("SELECT id,heading,text FROM chunks WHERE id>?1 ORDER BY id")?;
    let mut rows = s.query([after])?;
    while let Some(r) = rows.next()? {
        let (cid, heading, text): (i64, String, String) = (r.get(0)?, r.get(1)?, r.get(2)?);
        writer.add_document(doc!(id => cid as u64, body => format!("{heading}\n{text}")))?;
    }
    Ok(())
}
/// Open the derived index, creating it or appending the chunks it has not seen yet.
fn open(c: &Connection) -> Result<Option<Open>> {
    let Some(dir) = dir_of(c) else {
        return Ok(None);
    };
    let cipher_salt = cipher_salt(c)?;
    if cipher_salt.is_none() {
        footnote_core::no_links(&dir)?;
    }
    let seq = sequence(c)?;
    let (schema, id, body) = schema();
    let built = if cipher_salt.is_some() {
        None
    } else {
        read_built(&dir)
    };
    let index = if cipher_salt.is_some() {
        Index::create_in_ram(schema)
    } else {
        match built {
            Some(_) => Index::open_in_dir(&dir).context("open lexical index")?,
            None => {
                // Missing or half-written: start over from SQLite.
                if dir.exists() {
                    fs::remove_dir_all(&dir).context("reset lexical index")?;
                }
                crate::state::ensure_private_dir(&dir)?;
                Index::create_in_dir(&dir, schema).context("create lexical index")?
            }
        }
    };
    index.tokenizers().register(ANALYZER, analyzer());
    let built_seq = built.unwrap_or(0);
    if seq > built_seq || built.is_none() {
        let mut writer: IndexWriter = index
            .writer(WRITER_HEAP)
            .context("lexical index is locked")?;
        add_chunks(c, &mut writer, id, body, built_seq)?;
        writer.commit()?;
        if cipher_salt.is_none() {
            write_built(&dir, seq)?;
        }
    }
    let reader = index.reader()?;
    Ok(Some(Open {
        dir,
        built_seq: seq.max(built_seq),
        cipher_salt,
        index,
        reader,
        id,
        body,
    }))
}
/// Build or extend the derived index now, so the first query does not pay for it (at one million chunks that is about 35 s).
/// `Ok(None)`: this database has no derived index (in-memory).
pub fn sync(c: &Connection) -> Result<Option<i64>> {
    Ok(open(c)?.map(|o| o.built_seq))
}
/// Chunk ids and BM25 scores matching any of the query's words, best first. `Ok(None)`: no derived index here, use FTS5.
pub fn top_hits(c: &Connection, query: &str, n: usize) -> Result<Option<Vec<(i64, f32)>>> {
    let seq = sequence(c)?;
    let Some(dir) = dir_of(c) else {
        return Ok(None);
    };
    let cipher_salt = cipher_salt(c)?;
    let cell = OPEN.get_or_init(|| Mutex::new(None));
    let mut guard = cell
        .lock()
        .map_err(|_| anyhow::anyhow!("lexical cache poisoned"))?;
    let fresh = matches!(&*guard, Some(o) if o.dir == dir && o.built_seq == seq && o.cipher_salt == cipher_salt);
    if !fresh {
        *guard = open(c)?;
    }
    let Some(o) = guard.as_ref() else {
        return Ok(None);
    };
    o.reader.reload()?;
    let mut analyzer = o
        .index
        .tokenizers()
        .get(ANALYZER)
        .context("analyzer missing")?;
    let mut terms = BTreeSet::new();
    let mut stream = analyzer.token_stream(query);
    while stream.advance() {
        terms.insert(stream.token().text.clone());
    }
    if terms.is_empty() {
        return Ok(Some(vec![]));
    }
    let clauses: Vec<(Occur, Box<dyn Query>)> = terms
        .iter()
        .map(|t| {
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(o.body, t),
                    IndexRecordOption::WithFreqs,
                )) as Box<dyn Query>,
            )
        })
        .collect();
    let searcher = o.reader.searcher();
    let hits = searcher.search(
        &BooleanQuery::new(clauses),
        &TopDocs::with_limit(n.max(1)).order_by_score(),
    )?;
    let mut ids = Vec::with_capacity(hits.len());
    for (score, addr) in hits {
        let d: TantivyDocument = searcher.doc(addr)?;
        if let Some(v) = d.get_first(o.id).and_then(|v| v.as_u64()) {
            ids.push((v as i64, score));
        }
    }
    Ok(Some(ids))
}

/// Same as `top_hits`, ids only.
pub fn top_ids(c: &Connection, query: &str, n: usize) -> Result<Option<Vec<i64>>> {
    Ok(top_hits(c, query, n)?.map(|h| h.into_iter().map(|(id, _)| id).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(dir: &Path) -> Connection {
        let c = Connection::open(dir.join("k.db")).unwrap();
        c.execute_batch("CREATE TABLE chunks(id INTEGER PRIMARY KEY AUTOINCREMENT,heading TEXT NOT NULL,text TEXT NOT NULL);").unwrap();
        c
    }
    #[test]
    fn ranks_by_bm25_stems_and_folds_accents() {
        let t = tempfile::tempdir().unwrap();
        let c = db(t.path());
        for (h, x) in [
            ("a", "running dogs bark loudly"),
            ("b", "the cat sleeps"),
            ("c", "élève rapide"),
        ] {
            c.execute("INSERT INTO chunks(heading,text) VALUES(?1,?2)", [h, x])
                .unwrap();
        }
        assert_eq!(top_ids(&c, "dog runs", 5).unwrap().unwrap(), vec![1]);
        assert_eq!(top_ids(&c, "eleve", 5).unwrap().unwrap(), vec![3]);
        assert!(top_ids(&c, "???", 5).unwrap().unwrap().is_empty());
    }
    #[test]
    fn new_chunks_are_appended_without_a_rebuild() {
        let t = tempfile::tempdir().unwrap();
        let c = db(t.path());
        c.execute(
            "INSERT INTO chunks(heading,text) VALUES('a','alpha beta')",
            [],
        )
        .unwrap();
        assert_eq!(top_ids(&c, "alpha", 5).unwrap().unwrap(), vec![1]);
        c.execute(
            "INSERT INTO chunks(heading,text) VALUES('b','alpha gamma gamma')",
            [],
        )
        .unwrap();
        let ids = top_ids(&c, "gamma", 5).unwrap().unwrap();
        assert_eq!(ids, vec![2]);
        assert_eq!(top_ids(&c, "alpha", 5).unwrap().unwrap().len(), 2);
    }
    #[test]
    fn in_memory_databases_have_no_derived_index() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE chunks(id INTEGER PRIMARY KEY AUTOINCREMENT,heading TEXT,text TEXT);",
        )
        .unwrap();
        assert!(top_ids(&c, "x", 5).unwrap().is_none());
    }
}
