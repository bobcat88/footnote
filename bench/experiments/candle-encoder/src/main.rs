//! #358: can a pure-Rust engine (candle, tract) give the same vectors as Footnote's ONNX Runtime encoder, and how fast?
//!
//! candle-encoder-bench <safetensors dir> <footnote model dir> <texts.json> <query|passage> <engines>
//!
//! - safetensors dir: the model's Hugging Face `config.json` and `model.safetensors` (BERT or ModernBERT), for candle.
//! - footnote model dir: `manifest.json`, `model.onnx`, `tokenizer.json`; Footnote's own encoder and tract load its `model.onnx`.
//! - query: each text encoded alone (latency); passage: all texts in one call, batches of 8 (throughput).
//! - engines: comma list of onnx, candle, tract. With onnx in the list, every other engine's vectors are compared with it
//!   (cosine per text, top-10 neighbour overlap).
//!
//! candle and tract copy Footnote's recipe: same tokenizer file, truncation, padding to the longest text of each batch of 8,
//! attention-mask mean pooling and L2 normalization in f64. Threads: RAYON_NUM_THREADS for candle and tract; Footnote's ONNX
//! query session uses 2 threads and its passage session half the logical CPUs.
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

#[cfg(any(feature = "candle", feature = "tract"))]
mod common {
    use anyhow::{Context, Error, Result};
    use std::{fs, path::Path};
    use tokenizers::{Encoding, PaddingParams, Tokenizer, TruncationParams};

    /// The model's tokenizer set up as Footnote's encoder sets it up.
    pub fn tokenizer(footnote_model: &Path) -> Result<Tokenizer> {
        let m: serde_json::Value =
            serde_json::from_slice(&fs::read(footnote_model.join("manifest.json"))?)?;
        let mut tok = Tokenizer::from_file(footnote_model.join("tokenizer.json")).map_err(Error::msg)?;
        tok.with_truncation(Some(TruncationParams {
            max_length: m["max_length"].as_u64().context("max_length")? as usize,
            ..Default::default()
        }))
        .map_err(Error::msg)?;
        let pad_token = m["pad_token"].as_str().unwrap_or("<pad>").to_string();
        let pad_id = tok.token_to_id(&pad_token).context("pad token")?;
        tok.with_padding(Some(PaddingParams {
            pad_id,
            pad_token,
            ..Default::default()
        }));
        Ok(tok)
    }

    /// Texts in batches of 8: ids and mask, row-major, with the padded length.
    pub fn batches(tok: &Tokenizer, texts: &[String]) -> Result<Vec<(usize, usize, Vec<u32>, Vec<u32>)>> {
        texts
            .chunks(8)
            .map(|part| {
                let t: Vec<Encoding> = tok.encode_batch(part.to_vec(), true).map_err(Error::msg)?;
                let (rows, len) = (t.len(), t[0].get_ids().len());
                let ids = t.iter().flat_map(|e| e.get_ids().to_vec()).collect();
                let mask = t.iter().flat_map(|e| e.get_attention_mask().to_vec()).collect();
                Ok((rows, len, ids, mask))
            })
            .collect()
    }

    /// Mean over real tokens, then L2, in f64. `hidden` is rows × len × dim, row-major.
    pub fn pool(hidden: &[f32], mask: &[u32], rows: usize, len: usize, out: &mut Vec<Vec<f64>>) {
        let dim = hidden.len() / (rows * len);
        for r in 0..rows {
            let (mut v, mut n) = (vec![0f64; dim], 0f64);
            for t in 0..len {
                if mask[r * len + t] != 0 {
                    n += 1.0;
                    let h = &hidden[(r * len + t) * dim..][..dim];
                    v.iter_mut().zip(h).for_each(|(a, &x)| *a += x as f64);
                }
            }
            v.iter_mut().for_each(|a| *a /= n);
            let norm = v.iter().map(|a| a * a).sum::<f64>().sqrt().max(1e-12);
            v.iter_mut().for_each(|a| *a /= norm);
            out.push(v);
        }
    }
}

#[cfg(feature = "candle")]
mod cand {
    use crate::common;
    use anyhow::Result;
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::{bert, modernbert};
    use std::{fs, path::Path};
    use tokenizers::Tokenizer;

    enum Net {
        Bert(bert::BertModel),
        Modern(modernbert::ModernBert),
    }
    pub struct Enc {
        tok: Tokenizer,
        net: Net,
    }

    impl Enc {
        pub fn load(weights: &Path, footnote_model: &Path) -> Result<Self> {
            let cfg = fs::read_to_string(weights.join("config.json"))?;
            let kind: serde_json::Value = serde_json::from_str(&cfg)?;
            // SAFETY: the weights file is not modified while mapped (a bench on local files).
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights.join("model.safetensors")], DType::F32, &Device::Cpu)?
            };
            let net = if kind["model_type"] == "modernbert" {
                // candle names the weights `model.*`; sentence-transformers exports drop that prefix.
                let vb = vb.rename_f(|s| s.strip_prefix("model.").unwrap_or(s).to_string());
                Net::Modern(modernbert::ModernBert::load(vb, &serde_json::from_str(&cfg)?)?)
            } else {
                Net::Bert(bert::BertModel::load(vb, &serde_json::from_str(&cfg)?)?)
            };
            Ok(Self { tok: common::tokenizer(footnote_model)?, net })
        }

        pub fn encode(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
            let mut out = Vec::with_capacity(texts.len());
            for (rows, len, ids, mask) in common::batches(&self.tok, texts)? {
                let ids_t = Tensor::from_vec(ids, (rows, len), &Device::Cpu)?;
                let mask_t = Tensor::from_vec(mask.clone(), (rows, len), &Device::Cpu)?;
                let hidden = match &self.net {
                    Net::Bert(n) => n.forward(&ids_t, &ids_t.zeros_like()?, Some(&mask_t))?,
                    Net::Modern(n) => n.forward(&ids_t, &mask_t)?,
                };
                let hidden = hidden.to_dtype(DType::F32)?.flatten_all()?.to_vec1::<f32>()?;
                common::pool(&hidden, &mask, rows, len, &mut out);
            }
            Ok(out)
        }
    }
}

#[cfg(feature = "tract")]
mod tr {
    use crate::common;
    use anyhow::{Result, ensure};
    use std::path::Path;
    use tokenizers::Tokenizer;
    use tract_onnx::prelude::*;

    pub struct Enc {
        tok: Tokenizer,
        plan: Arc<TypedSimplePlan>,
        inputs: Vec<String>,
    }

    impl Enc {
        pub fn load(footnote_model: &Path, threads: usize) -> Result<Self> {
            if threads > 1 {
                tract_linalg::multithread::set_default_executor(
                    tract_linalg::multithread::Executor::multithread(threads),
                );
            }
            let model = tract_onnx::onnx().model_for_path(footnote_model.join("model.onnx"))?;
            let inputs = model
                .input_outlets()?
                .iter()
                .map(|o| model.node(o.node).name.clone())
                .collect();
            let plan = model.into_optimized()?.into_runnable()?;
            Ok(Self { tok: common::tokenizer(footnote_model)?, plan, inputs })
        }

        pub fn encode(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
            let mut out = Vec::with_capacity(texts.len());
            for (rows, len, ids, mask) in common::batches(&self.tok, texts)? {
                let feed = |v: &[u32]| -> Result<TValue> {
                    let a = tract_ndarray::Array2::from_shape_vec((rows, len), v.iter().map(|&x| x as i64).collect())?;
                    Ok(a.into_tensor().into_tvalue())
                };
                let zeros = vec![0u32; ids.len()];
                let mut feeds = tvec!();
                for name in &self.inputs {
                    feeds.push(match name.as_str() {
                        "input_ids" => feed(&ids)?,
                        "attention_mask" => feed(&mask)?,
                        "token_type_ids" => feed(&zeros)?,
                        other => anyhow::bail!("unexpected model input: {other}"),
                    });
                }
                let res = self.plan.run(feeds)?;
                let hidden = res[0].to_plain_array_view::<f32>()?;
                ensure!(hidden.shape()[..2] == [rows, len], "unexpected output shape {:?}", hidden.shape());
                let flat: Vec<f32> = hidden.iter().copied().collect();
                common::pool(&flat, &mask, rows, len, &mut out);
            }
            Ok(out)
        }
    }
}

/// Peak working set of this process, in MiB (Windows).
fn peak_mib() -> f64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct Pmc {
            cb: u32,
            faults: u32,
            peak: usize,
            rest: [usize; 7],
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(h: isize, p: *mut Pmc, cb: u32) -> i32;
        }
        let mut p = Pmc { cb: std::mem::size_of::<Pmc>() as u32, ..Default::default() };
        // SAFETY: documented Win32 call with a correctly sized PROCESS_MEMORY_COUNTERS.
        if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut p, p.cb) } != 0 {
            return p.peak as f64 / 1048576.0;
        }
    }
    f64::NAN
}

/// Encodes with `f`, returning the vectors and a timing summary.
fn timed(
    kind: &str,
    texts: &[String],
    mut f: impl FnMut(&[String]) -> Result<Vec<Vec<f64>>>,
) -> Result<(Vec<Vec<f64>>, Value)> {
    f(&texts[..texts.len().min(8)])?; // warm-up
    if kind == "query" {
        let (mut vs, mut ms) = (vec![], vec![]);
        for t in texts {
            let s = Instant::now();
            vs.extend(f(std::slice::from_ref(t))?);
            ms.push(s.elapsed().as_secs_f64() * 1000.0);
        }
        ms.sort_by(f64::total_cmp);
        let p = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
        Ok((vs, json!({"median_ms": p(0.5), "p95_ms": p(0.95), "texts": texts.len()})))
    } else {
        let s = Instant::now();
        let vs = f(texts)?;
        let secs = s.elapsed().as_secs_f64();
        Ok((vs, json!({"seconds": secs, "texts_per_s": texts.len() as f64 / secs, "texts": texts.len()})))
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Cosine of each pair, and how many of the first 50 texts' 10 nearest neighbours the two engines share.
fn parity(a: &[Vec<f64>], b: &[Vec<f64>]) -> Value {
    let mut cos: Vec<f64> = a.iter().zip(b).map(|(x, y)| dot(x, y)).collect();
    cos.sort_by(f64::total_cmp);
    let top = |vs: &[Vec<f64>], q: usize| {
        let mut s: Vec<(f64, usize)> =
            (0..vs.len()).filter(|&i| i != q).map(|i| (dot(&vs[q], &vs[i]), i)).collect();
        s.sort_by(|x, y| y.0.total_cmp(&x.0));
        s.into_iter().take(10).map(|x| x.1).collect::<Vec<_>>()
    };
    let qs = a.len().min(50);
    let overlap: f64 = (0..qs)
        .map(|q| {
            let (ta, tb) = (top(a, q), top(b, q));
            ta.iter().filter(|i| tb.contains(i)).count() as f64 / 10.0
        })
        .sum::<f64>()
        / qs as f64;
    json!({
        "min": cos[0],
        "p01": cos[(cos.len() - 1) / 100],
        "mean": cos.iter().sum::<f64>() / cos.len() as f64,
        "below_0_9999": cos.iter().filter(|&&c| c < 0.9999).count(),
        "top10_overlap": overlap,
    })
}

type Engine = Box<dyn FnMut(&[String]) -> Result<Vec<Vec<f64>>>>;

/// Loads one engine; returns its encode function.
#[allow(unused_variables)]
fn load(name: &str, weights: &Path, model: &Path, kind: &str) -> Result<Engine> {
    let threads = std::env::var("RAYON_NUM_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let kind = kind.to_string();
    Ok(match name {
        #[cfg(feature = "onnx")]
        "onnx" => {
            let mut e = footnote_search::encoder::Encoder::load(model)?;
            Box::new(move |x: &[String]| Ok(e.encode(x, &kind)?.vectors))
        }
        #[cfg(feature = "candle")]
        "candle" => {
            let e = cand::Enc::load(weights, model)?;
            Box::new(move |x: &[String]| e.encode(x))
        }
        #[cfg(feature = "tract")]
        "tract" => {
            let e = tr::Enc::load(model, threads)?;
            Box::new(move |x: &[String]| e.encode(x))
        }
        other => anyhow::bail!("engine {other} unknown or not built in"),
    })
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        a.len() == 6,
        "usage: candle-encoder-bench <safetensors dir> <footnote model dir> <texts.json> <query|passage> <onnx,candle,tract>"
    );
    let (weights, model, kind) = (Path::new(&a[1]), Path::new(&a[2]), a[4].as_str());
    let texts: Vec<String> = serde_json::from_slice(&std::fs::read(&a[3])?)?;
    let mut report = json!({"kind": kind, "engines": a[5]});
    let mut base: Option<Vec<Vec<f64>>> = None;
    for name in a[5].split(',') {
        let s = Instant::now();
        let mut f = load(name, weights, model, kind)?;
        let load_ms = s.elapsed().as_secs_f64() * 1000.0;
        let (vs, mut t) = timed(kind, &texts, &mut f)?;
        t["load_ms"] = json!(load_ms);
        if let Some(b) = &base {
            t["parity_vs_onnx"] = parity(b, &vs);
        }
        report[name] = t;
        if name == "onnx" {
            base = Some(vs);
        }
    }
    report["peak_mib"] = json!(peak_mib());
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
