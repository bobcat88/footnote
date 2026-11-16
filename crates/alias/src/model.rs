//! Stage 2: names, addresses, dates and other free-form personal data, found by OpenAI Privacy Filter (Apache-2.0, ONNX q4,
//! revision 7ffa9a04). Same recipe as the benchmarked Python runner (bench/experiments/alias_pf.py): per-token logits over 33
//! BIOES classes, log-softmax, then a constrained Viterbi path (only legal BIOES transitions, every bias 0).
use crate::Hit;
use anyhow::{Context, Result, anyhow, bail};
use ort::session::Session;
use ort::value::Tensor;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokenizers::Tokenizer;

const CATS: [&str; 8] = ["ACCOUNT", "ADDRESS", "DATE", "EMAIL", "PERSON", "PHONE", "URL", "SECRET"];
const N: usize = 1 + 4 * CATS.len();
/// Pinned digests of the graph and tokenizer as benchmarked (ALIAS-EVAL.md).
const PINNED: [(&str, &str); 2] = [
    ("onnx/model_q4.onnx", "8f7dee8b46d096f052b359375dfba5d983cc4d18c44a783bf548615c472f8dea"),
    ("tokenizer.json", "0614fe83cadab421296e664e1f48f4261fa8fef6e03e63bb75c20f38e37d07d3"),
];
/// Longest text one call accepts (characters); longer input is refused rather than silently truncated.
const MAX_CHARS: usize = 100_000;

/// Tag `t`: 0 is O, then for each category B, I, E, S.
fn kind(t: usize) -> Option<(u8, usize)> {
    (t > 0).then(|| (b"BIES"[(t - 1) % 4], (t - 1) / 4))
}
fn allowed(a: usize, b: usize) -> bool {
    match (kind(a), kind(b)) {
        (None | Some((b'E' | b'S', _)), None) => true,
        (None | Some((b'E' | b'S', _)), Some((k, _))) => k == b'B' || k == b'S',
        (Some(_), Some((k, c))) => (k == b'I' || k == b'E') && Some(c) == kind(a).map(|x| x.1),
        (Some(_), None) => false,
    }
}
fn may_start(t: usize) -> bool {
    kind(t).is_none_or(|(k, _)| k == b'B' || k == b'S')
}
fn may_end(t: usize) -> bool {
    kind(t).is_none_or(|(k, _)| k == b'E' || k == b'S')
}

/// Best legal tag path for rows of log-probabilities (`N` per token).
fn viterbi(logp: &[f32]) -> Vec<usize> {
    let rows: Vec<&[f32]> = logp.chunks(N).collect();
    let mut score: Vec<f32> = (0..N).map(|t| if may_start(t) { rows[0][t] } else { f32::NEG_INFINITY }).collect();
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(rows.len());
    for row in &rows[1..] {
        let mut next = vec![f32::NEG_INFINITY; N];
        let mut from = vec![0; N];
        for b in 0..N {
            for a in (0..N).filter(|&a| allowed(a, b)) {
                if score[a] > next[b] {
                    (next[b], from[b]) = (score[a], a);
                }
            }
            next[b] += row[b];
        }
        back.push(from);
        score = next;
    }
    let mut t = (0..N).filter(|&t| may_end(t)).max_by(|&a, &b| score[a].total_cmp(&score[b])).unwrap_or(0);
    let mut path = vec![t];
    for from in back.iter().rev() {
        t = from[t];
        path.push(t);
    }
    path.reverse();
    path
}

pub struct PrivacyFilter {
    session: Session,
    tokenizer: Tokenizer,
}

impl PrivacyFilter {
    /// Load from a directory holding `onnx/model_q4.onnx`, its `model_q4.onnx_data` and `tokenizer.json`.
    pub fn load(dir: &Path) -> Result<Self> {
        for (file, want) in PINNED {
            let bytes = std::fs::read(dir.join(file)).with_context(|| format!("read {}", dir.join(file).display()))?;
            if format!("{:x}", Sha256::digest(&bytes)) != want {
                bail!("{file} does not match the benchmarked Privacy Filter");
            }
        }
        // shortcut: the 875 MB weights file is not hashed (seconds per load) and the graph is re-read by path after the
        // check; acceptable while the model sits in the per-user state folder; pin it at install time when a recipe exists.
        let tokenizer = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| anyhow!("tokenizer: {e}"))?;
        // shortcut: its own ONNX Runtime environment; share the embedder's once Alias and embeddings run in one process.
        let env = ort::init().with_telemetry(false).build().map_err(|e| anyhow!("{e}"))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(2, 4);
        let err = |e: ort::Error<_>| anyhow!("load Privacy Filter: {e}");
        let session = Session::builder(&env)
            .map_err(|e| anyhow!("load Privacy Filter: {e}"))?
            .with_intra_threads(threads)
            .map_err(err)?
            .with_inter_threads(1)
            .map_err(err)?
            .commit_from_file(dir.join("onnx/model_q4.onnx"))
            .map_err(|e| anyhow!("load Privacy Filter: {e}"))?;
        Ok(Self { session, tokenizer })
    }

    /// Spans the model tags, in character offsets. Account numbers are left to the check-digit rules.
    pub fn detect(&mut self, text: &str) -> Result<Vec<Hit>> {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() > MAX_CHARS {
            bail!("text longer than {MAX_CHARS} characters");
        }
        let enc = self.tokenizer.encode_char_offsets(text, true).map_err(|e| anyhow!("tokenize: {e}"))?;
        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| i64::from(x)).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let len = ids.len();
        let out = self
            .session
            .run(ort::inputs![
                "input_ids" => Tensor::from_array(([1, len], ids)).map_err(|e| anyhow!("{e}"))?,
                "attention_mask" => Tensor::from_array(([1, len], vec![1i64; len])).map_err(|e| anyhow!("{e}"))?,
            ])
            .map_err(|e| anyhow!("run Privacy Filter: {e}"))?;
        let (shape, logits) = out[0].try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
        if shape.len() != 3 || shape[1] as usize != len || shape[2] as usize != N {
            bail!("unexpected Privacy Filter output shape {shape:?}");
        }
        let mut logp = logits.to_vec();
        for row in logp.chunks_mut(N) {
            let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let lse = max + row.iter().map(|x| (x - max).exp()).sum::<f32>().ln();
            row.iter_mut().for_each(|x| *x -= lse);
        }
        let mut spans: Vec<(usize, usize, usize)> = Vec::new();
        let mut open = false;
        for (&(s, e), t) in enc.get_offsets().iter().zip(viterbi(&logp)) {
            let Some((k, c)) = kind(t) else {
                open = false;
                continue;
            };
            match spans.last_mut() {
                Some(last) if open && (k == b'I' || k == b'E') => last.1 = e,
                _ => spans.push((s, e, c)),
            }
            open = k == b'B' || k == b'I';
        }
        Ok(spans
            .into_iter()
            .filter(|&(_, _, c)| CATS[c] != "ACCOUNT")
            .filter_map(|(mut s, e, c)| {
                // Byte-level BPE offsets carry the leading space of a word.
                while s < e && chars[s].is_whitespace() {
                    s += 1;
                }
                (s < e).then(|| Hit { start: s, end: e, label: CATS[c], value: chars[s..e].iter().collect() })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viterbi_never_takes_an_illegal_transition() {
        // Row-wise argmax would give O, I-PERSON, O (illegal: I without B); the constrained path must not.
        let mut logp = vec![-10.0f32; 3 * N];
        logp[0] = 0.0;
        logp[N + 1 + 4 * 4 + 1] = 0.0;
        logp[2 * N] = 0.0;
        let path = viterbi(&logp);
        for w in path.windows(2) {
            assert!(allowed(w[0], w[1]), "{path:?}");
        }
        assert!(may_start(path[0]) && may_end(*path.last().unwrap()));
    }
}
