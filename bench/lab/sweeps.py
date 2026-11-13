"""Named configuration families. Each returns [(label, config)] for a dataset. Keep them small and meaningful: every row costs minutes."""
from __future__ import annotations

EN_MODELS = ["bge-small-en", "gte-small", "minilm-l6-en"]
ML_MODELS = ["minilm-l12-multi", "e5-small-multi", "bekko-a8m", "bekko-a25m", "granite-97m-multi"]
DENSE_ONLY = {"lexical": {"on": False}, "fusion": {"kind": "dense"}, "chunker": {"kind": "whole"}}


def models(ds: str, base: dict):
    """Dense-only, whole documents (the cleanest model comparison; matches how MTEB scores are produced)."""
    return [(f"dense:{m}", {**DENSE_ONLY, "dense": {"model": m}}) for m in EN_MODELS + ML_MODELS]


def lexical(ds: str, base: dict):
    """BM25 tokenization and parameters on whole documents."""
    out = []
    for stem in ("none", "en", "fr", "both"):
        out.append((f"bm25:stem={stem}", {"dense": {"on": False}, "fusion": {"kind": "lexical"}, "chunker": {"kind": "whole"},
                                          "lexical": {"tokenizer": {"accents": "strip", "stem": stem}}}))
    for k1, b in ((0.9, 0.4), (1.2, 0.75), (1.5, 0.75), (2.0, 0.75), (1.2, 0.9)):
        out.append((f"bm25:stem=both,k1={k1},b={b}", {"dense": {"on": False}, "fusion": {"kind": "lexical"}, "chunker": {"kind": "whole"},
                                                       "lexical": {"k1": k1, "b": b, "tokenizer": {"accents": "strip", "stem": "both"}}}))
    return out


def fusion(ds: str, base: dict):
    """Fusion of BM25 and dense over the base config's model/tokenizer: single signals, RRF constants, interpolation weights."""
    def v(label, **over):
        from lab import merge  # noqa: PLC0415
        return label, merge(base, over)
    out = [v("dense-only", fusion={"kind": "dense"}), v("lexical-only", fusion={"kind": "lexical"})]
    for k in (10, 60):
        out.append(v(f"rrf k={k}", fusion={"kind": "rrf", "k": k}))
    for norm in ("minmax", "zscore"):
        for a in (0.5, 0.6, 0.7, 0.8, 0.9):
            out.append(v(f"cc {norm} alpha={a}", fusion={"kind": "cc", "norm": norm, "alpha": a}))
    return out


def chunking(ds: str, base: dict):
    """Chunk strategy and size, plus the encoder's token window. Documents shorter than the size are unaffected by design."""
    from lab import merge  # noqa: PLC0415
    out = [("whole", merge(base, {"chunker": {"kind": "whole"}}))]
    for kind in ("chars", "sent", "para"):
        for size in (500, 1000, 2000):
            if kind == "para" and size == 500:
                continue
            out.append((f"{kind} {size}", merge(base, {"chunker": {"kind": kind, "size": size, "overlap": 0}})))
    out.append(("chars 1000 overlap 200", merge(base, {"chunker": {"kind": "chars", "size": 1000, "overlap": 200}})))
    out.append(("sent 1000 +title", merge(base, {"chunker": {"kind": "sent", "size": 1000, "title_prefix": True}})))
    for ml in (128, 256, 512):
        out.append((f"sent 1000 max_len={ml}", merge(base, {"chunker": {"kind": "sent", "size": 1000}, "dense": {"max_len": ml}})))
    return out


def rerank(ds: str, base: dict):
    """Second-stage cross-encoder over the top pool: quality gain versus the cost in candidates scored."""
    from lab import merge  # noqa: PLC0415
    out = [("no rerank", merge(base, {"rerank": {"on": False}}))]
    for model in ("mxbai-xsmall", "mmarco-mminilm", "bge-reranker-v2-m3"):
        for pool in (5, 10, 20):
            out.append((f"{model} pool={pool}", merge(base, {"rerank": {"on": True, "model": model, "pool": pool}})))
    return out


def lexical_lang(ds: str, base: dict):
    """Stemming policy on French / mixed text: none, one fixed language, or the language detected for each text."""
    out = []
    for stem in ("none", "en", "fr", "auto"):
        for k1 in (1.2, 1.5):
            out.append((f"bm25 stem={stem} k1={k1}", {"dense": {"on": False}, "fusion": {"kind": "lexical"}, "chunker": {"kind": "whole"},
                                                      "lexical": {"k1": k1, "tokenizer": {"accents": "strip", "stem": stem}}}))
    return out


def crowding(ds: str, base: dict):
    """Mixed-language index: does removing each language's mean embedding offset (dense.center = lang) reduce cross-language crowding?"""
    from lab import merge  # noqa: PLC0415
    out = []
    for center in ("none", "lang"):
        out.append((f"dense-only center={center}", merge(base, {"fusion": {"kind": "dense"}, "dense": {"center": center}})))
        out.append((f"hybrid center={center}", merge(base, {"dense": {"center": center}})))
    for stem in ("none", "both", "auto"):
        out.append((f"hybrid stem={stem}", merge(base, {"lexical": {"tokenizer": {"accents": "strip", "stem": stem}}})))
    return out


def experiments(ds: str, base: dict):
    """Pre-registered optimization experiments (see bench/experiments/PLAN.md). Every row varies ONE thing from the base config."""
    from lab import merge  # noqa: PLC0415
    v = lambda label, **over: (label, merge(base, over))
    out = [v("E00 base")]
    out += [v("E01 bm25 k1=1.2 b=0.75 (Tantivy fixed)", lexical={"k1": 1.2, "b": 0.75}),
            v("E01 bm25 k1=2.0 b=0.75", lexical={"k1": 2.0, "b": 0.75}),
            v("E01 bm25 k1=1.5 b=0.5", lexical={"k1": 1.5, "b": 0.5})]
    out += [v(f"E02 drop query words with idf<{t}", lexical={"min_idf": t}) for t in (1.0, 2.0, 3.0)]
    out += [v(f"E03 lexical PRF docs={d} terms={t} w={w}", lexical={"prf": {"docs": d, "terms": t, "weight": w}})
            for d, t, w in ((3, 8, 0.3), (5, 10, 0.5), (5, 20, 0.3))]
    out += [v(f"E04 title boost x{n}", lexical={"title_boost": n}) for n in (2, 3)]
    out += [v(f"E05 dense PRF docs={d} beta={b}", dense={"prf": {"docs": d, "beta": b}}) for d, b in ((3, 0.3), (3, 0.6), (5, 0.5))]
    out += [v("E06 adaptive alpha 0.4..0.8", fusion={"adaptive": {"on": True, "lo": 0.4, "hi": 0.8}}),
            v("E06 adaptive alpha 0.3..0.9", fusion={"adaptive": {"on": True, "lo": 0.3, "hi": 0.9}})]
    out += [v(f"E07 fusion pool={n}", fusion={"pool": n}) for n in (20, 50, 200)]
    out += [v(f"E08 two-stage dense on lexical top-{n}", fusion={"dense_from_lexical": n}) for n in (100, 1000)]
    out += [v("E09 vectors f16", dense={"quant": "f16"}), v("E10 vectors int8", dense={"quant": "int8"})]
    out += [v(f"E11 dims={d}", dense={"dims": d}) for d in (256, 192, 128)]
    return out


def ensemble(ds: str, base: dict):
    """E16: blend the cosine scores of two dense models (twice the encode cost) before fusion with BM25."""
    from lab import merge  # noqa: PLC0415
    out = [("E16 base (one model)", merge(base, {}))]
    for model in ("e5-small-multi", "minilm-l12-multi", "gte-small"):
        for w in (0.3, 0.5):
            out.append((f"E16 + {model} weight={w}", merge(base, {"dense": {"extra": {"model": model, "weight": w}}})))
    return out


def gated_rerank(ds: str, base: dict):
    """E12: bge-reranker-v2-m3 only where the dense ranking is not decisive (small cosine gap between the two best documents)."""
    from lab import merge  # noqa: PLC0415
    out = [("E12 no rerank", merge(base, {"rerank": {"on": False}}))]
    for pool in (10,):
        for gate in (0.0, 0.01, 0.03, 0.06):
            out.append((f"E12 bge-reranker-v2-m3 pool={pool} gate={gate}",
                        merge(base, {"rerank": {"on": True, "model": "bge-reranker-v2-m3", "pool": pool, "gate_margin": gate}})))
    return out


def fusion2(ds: str, base: dict):
    """Research-driven fusion experiments (bench/research/fusion-boost.md) on the shipped ensemble base."""
    from lab import merge  # noqa: PLC0415
    v = lambda label, **over: (label, merge(base, over))
    out = [v("F0 base (ensemble, cc z-score 0.55, floored)")]
    for a in (0.6, 0.65, 0.7):  # control: is a gain from completion, or just from more dense weight?
        out.append(v(f"F0 control floored alpha={a}", fusion={"alpha": a}))
    for a in (0.5, 0.55, 0.6, 0.65, 0.7):
        out.append(v(f"F1 exact score completion alpha={a}", fusion={"complete": True, "alpha": a}))
    for w in (0.3, 0.4, 0.6, 0.7):
        out.append(v(f"F3 second-model weight={w}", dense={"extra": {"model": "gte-small", "weight": w}}))
    for scale in (5.0, 10.0, 20.0):
        for a in (0.5, 0.6, 0.7):
            out.append(v(f"F4 atan BM25 squash s={scale} alpha={a}", fusion={"squash": scale, "alpha": a}))
    return out


SWEEPS = {"fusion2": fusion2, "ensemble": ensemble, "experiments": experiments, "gated_rerank": gated_rerank, "crowding": crowding, "models": models, "lexical": lexical, "fusion": fusion, "chunking": chunking, "rerank": rerank,
          "lexical_lang": lexical_lang}
