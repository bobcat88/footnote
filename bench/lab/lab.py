"""Lab engine: resident ONNX encoders + own BM25 + chunkers + fusion, all in-process.

It exists to explore the design space quickly (milliseconds per query instead of a process per query) with the same models and the same
math the product uses or could use. Winners are ported to the Rust core; the product engines in engines.py remain the reference.

Config (all keys optional, defaults shown in `DEFAULT`):
  chunker: {kind: whole|chars|sent|para, size: 2000, overlap: 0, title_prefix: false}
  lexical: {on: true, tokenizer: {accents: strip|keep, stem: none|en|fr|both}, k1: 1.2, b: 0.75}
  dense:   {on: true, model: <name in models.MODELS>, max_len: <override>}
  fusion:  {kind: rrf|cc|dense|lexical, k: 60, alpha: 0.5, norm: minmax|zscore, pool: 100}
"""
from __future__ import annotations

import collections
import copy
import hashlib
import json
import math
import os
import re
import sys
import time
import unicodedata
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import models  # noqa: E402

CACHE = models.BENCH / ".cache" / "emb"
DEFAULT = {
    "chunker": {"kind": "chars", "size": 2000, "overlap": 0, "title_prefix": False},
    "lexical": {"on": True, "tokenizer": {"accents": "strip", "stem": "none"}, "k1": 1.2, "b": 0.75,
                "min_idf": 0.0, "prf": {"docs": 0, "terms": 10, "weight": 0.5}, "title_boost": 1},
    "dense": {"on": True, "model": "e5-small-multi", "max_len": None, "center": "none",
              "quant": "f32", "dims": 0, "prf": {"docs": 0, "beta": 0.5},
              "extra": {"model": "", "weight": 0.5}},
    "fusion": {"kind": "rrf", "k": 60, "alpha": 0.5, "norm": "minmax", "pool": 100, "dense_from_lexical": 0,
               "adaptive": {"on": False, "lo": 0.4, "hi": 0.8}},
    "rerank": {"on": False, "model": "mmarco-mminilm", "pool": 20, "gate_margin": 0.0},
}


def merge(base: dict, over: dict) -> dict:
    out = copy.deepcopy(base)
    for k, v in over.items():
        out[k] = merge(out[k], v) if isinstance(v, dict) and isinstance(out.get(k), dict) else v
    return out


# ------------------------------------------------------------------ chunking
SENT = re.compile(r"(?<=[.!?…])\s+|\n+")


def _accumulate(pieces: list[str], size: int, joiner: str) -> list[str]:
    out, cur = [], ""
    for p in pieces:
        if cur and len(cur) + len(joiner) + len(p) > size:
            out.append(cur)
            cur = ""
        cur = p if not cur else cur + joiner + p
        while len(cur) > size * 2:  # a single piece far above the size: hard cut so nothing is lost
            out.append(cur[:size])
            cur = cur[size:]
    if cur:
        out.append(cur)
    return out


TURN = re.compile(r"\n\n(?=(?:user|assistant): )")
PARA = "\n\n"


def chunk(title: str, text: str, cfg: dict) -> list[str]:
    kind, size, overlap = cfg["kind"], cfg["size"], cfg.get("overlap", 0)
    if cfg.get("roles") == "user":  # chat memory: index what the user said (LongMemEval paper: keys use user-side utterances)
        text = PARA.join(x for x in TURN.split(text) if x.startswith("user:"))
    if kind == "rounds":  # one user turn plus the assistant reply(ies) that follow it, packed up to `size`
        rounds, cur = [], []
        for turn in (x for x in TURN.split(text) if x.strip()):
            if turn.startswith("user:") and cur:
                rounds.append(PARA.join(cur))
                cur = []
            cur.append(turn)
        if cur:
            rounds.append(PARA.join(cur))
        return _accumulate(rounds, size, PARA) or [text]
    prefix = f"{title}\n" if (title and cfg.get("title_prefix")) else ""
    body = text if cfg.get("title_prefix") else (f"{title}\n{text}" if title else text)
    if kind == "whole" or len(body) <= size:
        parts = [body]
    elif kind == "chars":
        step = max(1, size - overlap)
        parts = [body[i:i + size] for i in range(0, len(body), step)]
    elif kind == "sent":
        parts = _accumulate([s for s in SENT.split(body) if s.strip()], size, " ")
    elif kind == "para":
        paras = [p for p in re.split(r"\n\s*\n", body) if p.strip()]
        parts = []
        for p in paras:
            parts += _accumulate([s for s in SENT.split(p) if s.strip()], size, " ") if len(p) > size else [p]
        parts = _accumulate(parts, size, "\n\n")
    else:
        raise ValueError(f"unknown chunker {kind}")
    return [(prefix + p) if (prefix and i > 0) else p for i, p in enumerate(parts)] if prefix else parts


# ------------------------------------------------------------------ language (for mixed indexes)
_FR = set("le la les des du de un une et est que qui dans pour pas sur au aux avec ce ces sont elle nous vous ils il en à ne se son sa "
          "ses plus par comme mais ou où quel quelle quels quelles comment pourquoi quand cette être avoir fait peut".split())
_EN = set("the of and to in is that for with as on are by this from it be or an at which how what why when where who not can does do "
          "has have was were will would their there these those".split())


def lang_of(text: str) -> str:
    """Cheap function-word vote between French and English: enough to label whole texts, wrong on very short ones."""
    words = re.findall(r"[^\W\d_]+", text.lower())[:300]
    fr, en = sum(w in _FR for w in words), sum(w in _EN for w in words)
    return "fr" if fr > en else "en" if en > fr else "other"


def center_by_language(emb: np.ndarray, labels: list[str]) -> tuple[np.ndarray, dict]:
    """Subtract each language's mean vector (the 'language-mean removal' baseline of the language-crowding literature), renormalize."""
    means = {l: emb[[i for i, x in enumerate(labels) if x == l]].mean(0) for l in set(labels)}
    out = emb - np.stack([means[l] for l in labels])
    return out / np.maximum(np.linalg.norm(out, axis=1, keepdims=True), 1e-12), means


# ------------------------------------------------------------------ lexical
_STEM: dict = {}


def _stemmer(lang: str):
    import snowballstemmer
    if lang not in _STEM:
        _STEM[lang] = snowballstemmer.stemmer({"en": "english", "fr": "french"}[lang])
    return _STEM[lang]


def make_tokenizer(cfg: dict):
    strip, stem = cfg.get("accents", "strip") == "strip", cfg.get("stem", "none")
    langs = {"none": [], "en": ["en"], "fr": ["fr"], "both": ["en", "fr"], "auto": []}[stem]
    cache: dict[str, tuple[str, ...]] = {}

    def fold(text: str) -> str:
        text = text.lower()
        if strip:
            text = "".join(c for c in unicodedata.normalize("NFKD", text) if not unicodedata.combining(c))
        return text

    def tokens(text: str) -> list[str]:
        out: list[str] = []
        words = re.findall(r"[^\W_]+", fold(text))
        if stem == "auto":  # stem the whole text in the language it appears to be written in; unknown language: leave as is
            lang = lang_of(text)
            if lang not in ("fr", "en"):
                return words
            stemmer = _stemmer(lang)
            return [cache.setdefault((lang, w), (stemmer.stemWord(w),))[0] for w in words]
        for w in words:
            if not langs:
                out.append(w)
                continue
            if w not in cache:
                cache[w] = tuple(dict.fromkeys(_stemmer(l).stemWord(w) for l in langs))
            out.extend(cache[w])
        return out

    return tokens


class Bm25:
    def __init__(self, docs: list[list[str]], k1: float = 1.2, b: float = 0.75) -> None:
        self.k1, self.b, self.n = k1, b, len(docs)
        self.dl = np.array([len(d) for d in docs], dtype=np.float32)
        self.avgdl = float(self.dl.mean()) if self.n else 1.0
        post: dict[str, tuple[list[int], list[int]]] = collections.defaultdict(lambda: ([], []))
        for i, toks in enumerate(docs):
            for t, c in collections.Counter(toks).items():
                post[t][0].append(i)
                post[t][1].append(c)
        self.post = {t: (np.array(d, dtype=np.int32), np.array(c, dtype=np.float32)) for t, (d, c) in post.items()}
        self.idf = {t: math.log(1 + (self.n - len(d) + 0.5) / (len(d) + 0.5)) for t, (d, _) in self.post.items()}
        self.norm = k1 * (1 - b + b * self.dl / self.avgdl)

    def scores(self, query_tokens: list[str], weights: dict[str, float] | None = None) -> np.ndarray:
        s = np.zeros(self.n, dtype=np.float32)
        for t in set(query_tokens):
            p = self.post.get(t)
            if p is None:
                continue
            d, tf = p
            w = 1.0 if weights is None else weights.get(t, 1.0)
            s[d] += w * self.idf[t] * (tf * (self.k1 + 1)) / (tf + self.norm[d])
        return s


# ------------------------------------------------------------------ dense
class Encoder:
    def __init__(self, name: str, max_len: int | None = None, threads: int | None = None) -> None:
        import onnxruntime as ort
        from tokenizers import Tokenizer
        self.name, self.spec = name, models.MODELS[name]
        self.max_len = max_len or self.spec["max_len"]
        folder = models.model_dir(name)
        self.tok = Tokenizer.from_file(str(folder / "tokenizer.json"))
        self.tok.enable_truncation(max_length=self.max_len)
        pad = next((t for t in ("<pad>", "[PAD]", "<|pad|>", "<|endoftext|>") if self.tok.token_to_id(t) is not None), None)
        self.tok.enable_padding(pad_id=self.tok.token_to_id(pad) if pad else 0, pad_token=pad or "[PAD]")
        so = ort.SessionOptions()
        so.intra_op_num_threads = threads or max(1, (os.cpu_count() or 4) // 2)
        self.sess = ort.InferenceSession(str(folder / "model.onnx"), so, providers=["CPUExecutionProvider"])
        self.inputs = {i.name for i in self.sess.get_inputs()}

    def embed(self, texts: list[str], kind: str, batch: int = 16) -> np.ndarray:
        prefix = self.spec["q" if kind == "query" else "p"]
        order = np.argsort([len(t) for t in texts], kind="stable")
        out = None
        for i in range(0, len(texts), batch):
            idx = order[i:i + batch]
            enc = self.tok.encode_batch([prefix + texts[j] for j in idx])
            ids = np.array([e.ids for e in enc], dtype=np.int64)
            mask = np.array([e.attention_mask for e in enc], dtype=np.int64)
            feed = {"input_ids": ids, "attention_mask": mask}
            if "token_type_ids" in self.inputs:
                feed["token_type_ids"] = np.zeros_like(ids)
            pooling = self.spec["pooling"]
            if pooling == "graph":  # pooling and projection are inside the ONNX graph (EmbeddingGemma)
                h = self.sess.run([self.spec.get("output", "sentence_embedding")], feed)[0]
            else:
                h = self.sess.run(None, feed)[0]
            if h.ndim == 3:
                if pooling == "cls":
                    h = h[:, 0]
                elif pooling == "last":  # last non-padding token (right padding)
                    h = h[np.arange(len(h)), mask.sum(1) - 1]
                else:
                    m = mask[..., None].astype(np.float32)
                    h = (h * m).sum(1) / np.maximum(m.sum(1), 1e-9)
            h = h / np.maximum(np.linalg.norm(h, axis=1, keepdims=True), 1e-12)
            if out is None:
                out = np.zeros((len(texts), h.shape[1]), dtype=np.float32)
            out[idx] = h
        return out if out is not None else np.zeros((0, 1), dtype=np.float32)


class CrossEncoder:
    """Pairwise relevance scorer (query, passage) -> one logit. Used as a second stage over a small candidate pool."""

    def __init__(self, name: str, threads: int | None = None) -> None:
        import onnxruntime as ort
        from tokenizers import Tokenizer
        self.name, self.spec = name, models.RERANKERS[name]
        folder = models.model_dir(name)
        self.tok = Tokenizer.from_file(str(folder / "tokenizer.json"))
        self.tok.enable_truncation(max_length=self.spec["max_len"])
        pad = next((t for t in ("<pad>", "[PAD]", "<|pad|>") if self.tok.token_to_id(t) is not None), None)
        self.tok.enable_padding(pad_id=self.tok.token_to_id(pad) if pad else 0, pad_token=pad or "[PAD]")
        so = ort.SessionOptions()
        so.intra_op_num_threads = threads or max(1, (os.cpu_count() or 4) // 2)
        self.sess = ort.InferenceSession(str(folder / "model.onnx"), so, providers=["CPUExecutionProvider"])
        self.inputs = {i.name for i in self.sess.get_inputs()}

    def score(self, query: str, passages: list[str], batch: int = 8) -> np.ndarray:
        out = np.zeros(len(passages), dtype=np.float32)
        for i in range(0, len(passages), batch):
            enc = self.tok.encode_batch([(query, p) for p in passages[i:i + batch]])
            ids = np.array([e.ids for e in enc], dtype=np.int64)
            feed = {"input_ids": ids, "attention_mask": np.array([e.attention_mask for e in enc], dtype=np.int64)}
            if "token_type_ids" in self.inputs:
                feed["token_type_ids"] = np.array([e.type_ids for e in enc], dtype=np.int64)
            logits = self.sess.run(None, feed)[0]
            out[i:i + batch] = logits[:, 0] if logits.ndim == 2 and logits.shape[1] == 1 else logits[:, -1] if logits.ndim == 2 else logits
        return out


RERANKERS: dict = {}


def get_reranker(name: str) -> CrossEncoder:
    if name not in RERANKERS:
        RERANKERS[name] = CrossEncoder(name)
    return RERANKERS[name]


def embed_key(enc: Encoder, texts: list[str], kind: str) -> str:
    """Cache key of an embedding matrix. Each text is length-prefixed: without that, two different segmentations of the same characters
    (whole document vs fixed-size cuts) concatenate to the same bytes and would share a key."""
    h = hashlib.sha256(json.dumps([enc.name, enc.max_len, kind, len(texts), models.lock_entry(enc.name)["files"]]).encode())
    for t in texts:
        b = t.encode()
        h.update(len(b).to_bytes(8, "little"))
        h.update(b)
    return h.hexdigest()


def cached_embed(enc: Encoder, texts: list[str], kind: str, use_cache: bool = True) -> np.ndarray:
    if not use_cache:
        return enc.embed(texts, kind)
    path = CACHE / f"{embed_key(enc, texts, kind)}.npy"
    if path.exists():
        return np.load(path)
    arr = enc.embed(texts, kind)
    CACHE.mkdir(parents=True, exist_ok=True)
    np.save(path, arr)
    return arr


# ------------------------------------------------------------------ fusion
def doc_scores(chunk_scores: np.ndarray, starts: np.ndarray) -> np.ndarray:
    return np.maximum.reduceat(chunk_scores, starts)


def _norm(values: np.ndarray, how: str) -> np.ndarray:
    if how == "zscore":
        sd = values.std() or 1.0
        return (values - values.mean()) / sd
    lo, hi = values.min(), values.max()
    return (values - lo) / (hi - lo) if hi > lo else np.zeros_like(values)


def fuse(lex: np.ndarray | None, den: np.ndarray | None, cfg: dict, k: int, alpha: float | None = None) -> list[int]:
    kind, pool = cfg["kind"], cfg["pool"]
    if kind == "dense" or lex is None:
        return list(np.argsort(-den, kind="stable")[:k])
    if kind == "lexical" or den is None:
        top = np.argsort(-lex, kind="stable")[:k]
        return [int(d) for d in top if lex[d] > 0]  # a document with no matching term is not a hit
    top_l = np.argsort(-lex, kind="stable")[:pool]
    top_l = top_l[lex[top_l] > 0]
    if len(top_l) == 0:  # no query term occurs in the corpus: the lexical signal is silent, fall back to the dense order
        return [int(d) for d in np.argsort(-den, kind="stable")[:k]]
    if cfg.get("dense_from_lexical"):  # two-stage: the dense signal only re-scores the lexical shortlist, no full dense scan
        short = np.argsort(-lex, kind="stable")[: cfg["dense_from_lexical"]]
        short = short[lex[short] > 0]
        top_d = short[np.argsort(-den[short], kind="stable")[:pool]]
    else:
        top_d = np.argsort(-den, kind="stable")[:pool]
    cand = np.union1d(top_l, top_d)
    if kind == "rrf":
        score = collections.defaultdict(float)
        for ranking in (top_l, top_d):
            for r, d in enumerate(ranking):
                score[int(d)] += 1.0 / (cfg["k"] + r + 1)
        order = sorted(score, key=lambda d: (-score[d], d))
        return order[:k]
    a = cfg["alpha"] if alpha is None else alpha
    if kind == "cc" and cfg.get("squash"):
        # query-independent scales: BM25 through atan(x / s), cosine as is (research: Vespa NFCorpus tutorial)
        s_ = float(cfg["squash"])
        score = {int(d): a * float(den[d]) + (1 - a) * float(np.arctan(lex[d] / s_)) * 2 / np.pi for d in cand}
        return sorted(score, key=lambda d: (-score[d], d))[:k]
    if kind == "cc" and cfg.get("complete"):
        # exact score completion: every candidate gets its real score from both retrievers, standardized with each pool's statistics
        def z(values: np.ndarray, pool_values: np.ndarray) -> np.ndarray:
            sd = float(pool_values.std()) or 1.0
            return (values - float(pool_values.mean())) / sd
        zl = z(lex[cand], lex[top_l])
        zd = z(den[cand], den[top_d])
        score = {int(d): a * float(vd) + (1 - a) * float(vl) for d, vl, vd in zip(cand, zl, zd)}
        return sorted(score, key=lambda d: (-score[d], d))[:k]
    if kind == "cc":
        nl = dict(zip(top_l.tolist(), _norm(lex[top_l], cfg["norm"])))
        nd = dict(zip(top_d.tolist(), _norm(den[top_d], cfg["norm"])))
        floor_l = min(nl.values()) - 1e-9 if cfg["norm"] == "zscore" else 0.0
        floor_d = min(nd.values()) - 1e-9 if cfg["norm"] == "zscore" else 0.0
        score = {int(d): a * nd.get(int(d), floor_d) + (1 - a) * nl.get(int(d), floor_l) for d in cand}
        return sorted(score, key=lambda d: (-score[d], d))[:k]
    raise ValueError(f"unknown fusion {kind}")


# ------------------------------------------------------------------ engine
ENCODERS: dict = {}  # one loaded model per (name, max_len), shared by every engine in the process


def get_encoder(name: str, max_len: int | None) -> Encoder:
    key = (name, max_len or models.MODELS[name]["max_len"])
    if key not in ENCODERS:
        ENCODERS[key] = Encoder(name, max_len)
    return ENCODERS[key]


class LabEngine:
    latency_scope = "in-process, resident model, numpy brute force (no process spawn): NOT comparable with CLI engines"

    def __init__(self, name: str, cfg: dict | None = None) -> None:
        self.cfg = merge(DEFAULT, cfg or {})
        self.id = f"lab:{name}"
        self.config = copy.deepcopy(self.cfg)
        self._enc: Encoder | None = None
        self._cache_embeddings = True

    def versions(self) -> dict:
        import onnxruntime, snowballstemmer, tokenizers
        v = {"numpy": np.__version__, "onnxruntime": onnxruntime.__version__, "tokenizers": tokenizers.__version__,
             "python_stem": "snowballstemmer"}
        if self.cfg["dense"]["on"]:
            v["model"] = self.cfg["dense"]["model"]
            v["model_lock"] = models.lock_entry(self.cfg["dense"]["model"])
        return v

    def close(self) -> None:
        pass

    def prepare(self, dataset: str, corpus: dict) -> dict:
        t0 = time.perf_counter()
        ccfg = self.cfg["chunker"]
        texts: list[str] = []
        doc_of: list[int] = []
        self.doc_ids = list(corpus)
        boost = self.cfg["lexical"]["title_boost"]
        lex_texts: list[str] = []
        for i, doc_id in enumerate(self.doc_ids):
            title, text = corpus[doc_id]
            for j, part in enumerate(chunk(title, text, ccfg)):
                texts.append(part)
                lex_texts.append((f"{title} " * (boost - 1) + part) if (boost > 1 and j == 0 and title) else part)
                doc_of.append(i)
        self.texts = texts
        self.starts = self._starts(doc_of)
        info = {"documents": len(corpus), "chunks": len(texts)}
        lex = self.cfg["lexical"]
        if lex["on"]:
            self.tokenize = make_tokenizer(lex["tokenizer"])
            chunk_tokens = [self.tokenize(t) for t in lex_texts]
            self.chunk_tokens = chunk_tokens if lex["prf"]["docs"] > 0 else None
            self.bm25 = Bm25(chunk_tokens, lex["k1"], lex["b"])
        embed_ms = 0.0
        if self.cfg["dense"]["on"]:
            e0 = time.perf_counter()
            self._enc = get_encoder(self.cfg["dense"]["model"], self.cfg["dense"]["max_len"])
            self._qcache: dict[str, np.ndarray] = {}
            self.emb = cached_embed(self._enc, texts, "passage", self._cache_embeddings)
            self.lang_means = {}
            if self.cfg["dense"].get("center") == "lang":
                self.emb, self.lang_means = center_by_language(self.emb, [lang_of(x) for x in texts])
            self.emb = self._quantize(self.emb)
            ex = self.cfg["dense"]["extra"]
            if ex["model"]:  # second encoder: its cosine scores are blended with the first one's before fusion
                self._enc2 = get_encoder(ex["model"], None)
                self.emb2 = cached_embed(self._enc2, texts, "passage", self._cache_embeddings)
                self._qcache2: dict[str, np.ndarray] = {}
            embed_ms = (time.perf_counter() - e0) * 1000
        info.update({"build_ms": (time.perf_counter() - t0) * 1000, "embed_ms": embed_ms, "disk_bytes": 0,
                     "index_bytes_in_memory": self._vector_bytes() if self.cfg["dense"]["on"] else 0})
        return info

    def _quantize(self, emb: np.ndarray) -> np.ndarray:
        """Store vectors the way a compact index would, then work on the dequantized values: this measures the ranking cost of the
        precision loss (speed and memory of real int8/f16 kernels are measured separately)."""
        d = self.cfg["dense"]
        if d["dims"]:
            emb = emb[:, : d["dims"]]
            emb = emb / np.maximum(np.linalg.norm(emb, axis=1, keepdims=True), 1e-12)
        if d["quant"] == "f16":
            emb = emb.astype(np.float16).astype(np.float32)
        elif d["quant"] == "int8":
            scale = np.maximum(np.abs(emb).max(axis=1, keepdims=True), 1e-12) / 127.0
            emb = (np.round(emb / scale).clip(-127, 127) * scale).astype(np.float32)
        return emb

    def _vector_bytes(self) -> int:
        per = {"f32": 4, "f16": 2, "int8": 1}[self.cfg["dense"]["quant"]]
        return int(self.emb.shape[0] * self.emb.shape[1] * per)

    def warm_queries(self, texts: list[str]) -> None:
        """Encode all queries in one batch (sweeps); search() then finds them cached."""
        if self.cfg["dense"]["on"]:
            todo = [t for t in dict.fromkeys(texts) if t not in self._qcache]
            for t, v in zip(todo, self._enc.embed(todo, "query")):
                self._qcache[t] = v
            if self.cfg["dense"]["extra"]["model"]:
                todo = [t for t in dict.fromkeys(texts) if t not in self._qcache2]
                for t, v in zip(todo, self._enc2.embed(todo, "query")):
                    self._qcache2[t] = v

    @staticmethod
    def _starts(doc_of: list[int]) -> np.ndarray:
        starts, last = [], -1
        for pos, d in enumerate(doc_of):
            if d != last:
                starts.append(pos)
                last = d
        return np.array(starts)

    def _lexical(self, text: str) -> np.ndarray:
        cfg = self.cfg["lexical"]
        qtok = self.tokenize(text)
        if cfg["min_idf"]:  # drop words that occur in a large share of the corpus (question words, function words)
            kept = [t for t in qtok if self.bm25.idf.get(t, 99.0) >= cfg["min_idf"]]
            qtok = kept or qtok
        scores = self.bm25.scores(qtok)
        prf = cfg["prf"]
        if prf["docs"] > 0 and self.chunk_tokens is not None and scores.max() > 0:
            top = np.argsort(-doc_scores(scores, self.starts), kind="stable")[: prf["docs"]]
            acc: collections.Counter = collections.Counter()
            for d in top:
                toks = self.chunk_tokens[int(self.starts[d])]
                for t, c in collections.Counter(toks).items():
                    acc[t] += c / max(len(toks), 1) * self.bm25.idf[t]
            have = set(qtok)
            extra = [t for t, _ in acc.most_common(prf["terms"] + len(have)) if t not in have][: prf["terms"]]
            scores = self.bm25.scores(qtok + extra, {t: prf["weight"] for t in extra})
        return scores

    def search(self, qid: str, text: str, k: int) -> list[str]:
        lex = den = lex_c = den_c = None
        if self.cfg["lexical"]["on"]:
            lex_c = self._lexical(text)
            lex = doc_scores(lex_c, self.starts)
        if self.cfg["dense"]["on"]:
            if text not in self._qcache:
                self._qcache[text] = self._enc.embed([text], "query")[0]
            qv = self._qcache[text]
            if self.lang_means:
                qv = qv - self.lang_means.get(lang_of(text), 0)
                qv = qv / max(float(np.linalg.norm(qv)), 1e-12)
            if self.cfg["dense"]["dims"]:
                qv = qv[: self.cfg["dense"]["dims"]]
                qv = qv / max(float(np.linalg.norm(qv)), 1e-12)
            den_c = self.emb @ qv
            den = doc_scores(den_c, self.starts)
            ex = self.cfg["dense"]["extra"]
            if ex["model"]:
                if text not in self._qcache2:
                    self._qcache2[text] = self._enc2.embed([text], "query")[0]
                den2 = doc_scores(self.emb2 @ self._qcache2[text], self.starts)
                z = lambda x: (x - x.mean()) / (x.std() or 1.0)
                den = (1 - ex["weight"]) * z(den) + ex["weight"] * z(den2)
            dprf = self.cfg["dense"]["prf"]
            if dprf["docs"] > 0:  # Rocchio: move the query toward the centroid of its best documents
                top = np.argsort(-den, kind="stable")[: dprf["docs"]]
                qv = qv + dprf["beta"] * self.emb[self.starts[top]].mean(axis=0)
                qv = qv / max(float(np.linalg.norm(qv)), 1e-12)
                den_c = self.emb @ qv
                den = doc_scores(den_c, self.starts)
        alpha = None
        ad = self.cfg["fusion"]["adaptive"]
        if ad["on"] and lex is not None and den is not None:  # a peaked lexical ranking earns more weight
            top = np.sort(lex)[::-1][:10]
            conf = float(1 - top[-1] / top[0]) if top[0] > 0 else 0.0
            alpha = ad["hi"] - (ad["hi"] - ad["lo"]) * conf
        rr = self.cfg["rerank"]
        use_rr = rr["on"]
        if use_rr and rr["gate_margin"] and den is not None:  # only spend the reranker where the dense ranking is not decisive
            top2 = np.sort(den)[::-1][:2]
            use_rr = bool(len(top2) > 1 and top2[0] - top2[1] < rr["gate_margin"])
        self.rr_total = getattr(self, "rr_total", 0) + 1
        self.rr_calls = getattr(self, "rr_calls", 0) + int(use_rr)
        order = fuse(lex, den, self.cfg["fusion"], max(k, rr["pool"]) if use_rr else k, alpha)
        if use_rr and order:
            head, tail = order[:rr["pool"]], order[rr["pool"]:]
            evidence = den_c if den_c is not None else lex_c  # rerank each candidate on its best-matching chunk
            ends = np.append(self.starts[1:], len(self.texts))
            passages = [self.texts[int(self.starts[d]) + int(np.argmax(evidence[self.starts[d]:ends[d]]))] for d in head]
            scores = get_reranker(rr["model"]).score(text, passages)
            head = [d for _, d in sorted(zip((-scores).tolist(), head), key=lambda x: (x[0], x[1]))]
            order = head + tail
        return [self.doc_ids[i] for i in order[:k]]
