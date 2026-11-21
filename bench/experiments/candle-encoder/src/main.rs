//! #358 step 1: does candle (pure Rust) give the same vectors as Footnote's ONNX Runtime encoder, and how fast?
//!
//! candle-encoder-bench <safetensors dir> <footnote model dir> <texts.json> <query|passage> <onnx|candle|both>
//!
//! - safetensors dir: the model's Hugging Face `config.json` and `model.safetensors` (BERT or ModernBERT).
//! - footnote model dir: `manifest.json`, `model.onnx`, `tokenizer.json`, loaded by Footnote's own encoder.
//! - query: each text encoded alone (latency); passage: all texts in one call, batches of 8 (throughput).
//! - both: also compares the two engines' vectors (cosine per text, top-10 neighbour overlap).
//!
//! The candle side copies Footnote's recipe: same tokenizer file, truncation, padding to the longest text of each batch of 8,
//! attention-mask mean pooling and L2 normalization in f64. Threads: RAYON_NUM_THREADS for candle; Footnote's ONNX query
//! session uses 2 threads and its passage session half the logical CPUs.
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

#[cfg(feature = "candle")]
mod cand {
    use anyhow::{Context, Error, Result};
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::{bert, modernbert};
    use std::{fs, path::Path};
    use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

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
            let m: serde_json::Value =
                serde_json::from_slice(&fs::read(footnote_model.join("manifest.json"))?)?;
            let mut tok =
                Tokenizer::from_file(footnote_model.join("tokenizer.json")).map_err(Error::msg)?;
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
            let cfg = fs::read_to_string(weights.join("config.json"))?;
            let kind: serde_json::Value = serde_json::from_str(&cfg)?;
            // SAFETY: the weights file is not modified while mapped (a bench on local files).
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(
                    &[weights.join("model.safetensors")],
                    DType::F32,
                    &Device::Cpu,
                )?
            };
            let net = if kind["model_type"] == "modernbert" {
                // candle names the weights `model.*`; sentence-transformers exports drop that prefix.
                let vb = vb.rename_f(|s| s.strip_prefix("model.").unwrap_or(s).to_string());
                Net::Modern(modernbert::ModernBert::load(vb, &serde_json::from_str(&cfg)?)?)
            } else {
                Net::Bert(bert::BertModel::load(vb, &serde_json::from_str(&cfg)?)?)
            };
            Ok(Self { tok, net })
        }

        pub fn encode(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
            let mut out = Vec::with_capacity(texts.len());
            for part in texts.chunks(8) {
                let tokens = self.tok.encode_batch(part.to_vec(), true).map_err(Error::msg)?;
                let (rows, len) = (tokens.len(), tokens[0].get_ids().len());
                let ids: Vec<u32> = tokens.iter().flat_map(|t| t.get_ids().to_vec()).collect();
                let mask: Vec<u32> = tokens
                    .iter()
                    .flat_map(|t| t.get_attention_mask().to_vec())
                    .collect();
                let ids_t = Tensor::from_vec(ids, (rows, len), &Device::Cpu)?;
                let mask_t = Tensor::from_vec(mask.clone(), (rows, len), &Device::Cpu)?;
                let hidden = match &self.net {
                    Net::Bert(n) => n.forward(&ids_t, &ids_t.zeros_like()?, Some(&mask_t))?,
                    Net::Modern(n) => n.forward(&ids_t, &mask_t)?,
                };
                let hidden = hidden.to_dtype(DType::F32)?.to_vec3::<f32>()?;
                for (r, rowh) in hidden.iter().enumerate() {
                    let dim = rowh[0].len();
                    let mut v = vec![0f64; dim];
                    let mut n = 0f64;
                    for (t, h) in rowh.iter().enumerate() {
                        if mask[r * len + t] != 0 {
                            n += 1.0;
                            v.iter_mut().zip(h).for_each(|(a, &x)| *a += x as f64);
                        }
                    }
                    v.iter_mut().for_each(|a| *a /= n);
                    let norm = v.iter().map(|a| a * a).sum::<f64>().sqrt().max(1e-12);
                    v.iter_mut().for_each(|a| *a /= norm);
                    out.push(v);
                }
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
        let mut p = Pmc {
            cb: std::mem::size_of::<Pmc>() as u32,
            ..Default::default()
        };
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
        let mut s: Vec<(f64, usize)> = (0..vs.len())
            .filter(|&i| i != q)
            .map(|i| (dot(&vs[q], &vs[i]), i))
            .collect();
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

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    anyhow::ensure!(a.len() == 6, "usage: candle-encoder-bench <safetensors dir> <footnote model dir> <texts.json> <query|passage> <onnx|candle|both>");
    let (weights, model, kind, engine) = (Path::new(&a[1]), Path::new(&a[2]), a[4].as_str(), a[5].as_str());
    let texts: Vec<String> = serde_json::from_slice(&std::fs::read(&a[3])?)?;
    let mut report = json!({"kind": kind, "engine": engine});
    let mut onnx_vs: Option<Vec<Vec<f64>>> = None;
    if engine != "candle" {
        #[cfg(feature = "onnx")]
        {
            let s = Instant::now();
            let mut e = footnote_search::encoder::Encoder::load(model)?;
            let load = s.elapsed().as_secs_f64() * 1000.0;
            let (vs, mut t) = timed(kind, &texts, |x| Ok(e.encode(x, kind)?.vectors))?;
            t["load_ms"] = json!(load);
            report["onnx"] = t;
            onnx_vs = Some(vs);
        }
        #[cfg(not(feature = "onnx"))]
        anyhow::bail!("built without the onnx feature");
    }
    if engine != "onnx" {
        #[cfg(feature = "candle")]
        {
            let s = Instant::now();
            let e = cand::Enc::load(weights, model)?;
            let load = s.elapsed().as_secs_f64() * 1000.0;
            let (vs, mut t) = timed(kind, &texts, |x| e.encode(x))?;
            t["load_ms"] = json!(load);
            report["candle"] = t;
            if let Some(o) = &onnx_vs {
                report["parity"] = parity(o, &vs);
            }
        }
        #[cfg(not(feature = "candle"))]
        anyhow::bail!("built without the candle feature");
    }
    let _ = (weights, &onnx_vs);
    report["peak_mib"] = json!(peak_mib());
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
