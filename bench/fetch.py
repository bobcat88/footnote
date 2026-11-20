"""Fetch public datasets, pin them by SHA-256 in datasets.lock.json, load them in one common shape.

First fetch records the hash (trust on first use, with the URL and size). Every later fetch or load verifies it
and fails hard on a mismatch: a dataset that changed under us would silently break comparability.
Common shape: corpus {doc_id: (title, text)}, queries {qid: text}, qrels {qid: {doc_id: relevance}}.
"""
from __future__ import annotations

import csv
import hashlib
import json
import sys
import urllib.request
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
CACHE = HERE / ".cache"          # downloads; ignored by git
LOCK = HERE / "datasets.lock.json"  # committed
BEIR = "https://public.ukp.informatik.tu-darmstadt.de/thakur/BEIR/datasets/{name}.zip"
HF = "https://huggingface.co/datasets/{repo}/resolve/{rev}/{file}"
DATASETS = {
    "scifact": {"kind": "beir", "url": BEIR.format(name="scifact"), "split": "test", "dev_file": "train"},
    "nfcorpus": {"kind": "beir", "url": BEIR.format(name="nfcorpus"), "split": "test", "dev_file": "dev"},
    # French tasks of MTEB(fra); same dataset revisions as the published anchors.
    "syntec": {"kind": "syntec", "repo": "lyon-nlp/mteb-fr-retrieval-syntec-s2p",
               "rev": "19661ccdca4dfc2d15122d776b61685f48c68ca9", "files": ["documents.json", "queries.json"],
               "split": "test"},
    # French + English documents in ONE index, queries of both languages: measures language crowding (a French question must find its
    # French document among English distractors and vice versa). Ids are prefixed `<part>__`.
    "mix-fr-en": {"kind": "union", "parts": ["alloprof", "syntec", "nfcorpus", "scifact"], "split": "test"},
    "longmemeval_s": {"kind": "longmemeval", "repo": "xiaowu0162/longmemeval-cleaned",
                      "rev": "98d7416c24c778c2fee6e6f3006e7a073259d48f", "files": ["longmemeval_s_cleaned.json"],
                      "split": "all"},
    "alloprof": {"kind": "mteb_parquet", "repo": "mteb/AlloprofRetrieval",
                 "rev": "96a52a1744e2d1981a66d93ff3b5a595f9a150d5",
                 "files": ["corpus/test-00000-of-00001.parquet", "queries/test-00000-of-00001.parquet",
                           "qrels/test-00000-of-00001.parquet"], "split": "test"},
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def read_lock() -> dict:
    return json.loads(LOCK.read_text(encoding="utf-8")) if LOCK.exists() else {}


def write_lock(lock: dict) -> None:
    tmp = LOCK.with_suffix(".tmp")
    tmp.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(LOCK)


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(url, headers={"User-Agent": "footnote-bench/1"})
    with urllib.request.urlopen(request, timeout=120) as response, dest.open("wb") as out:
        while block := response.read(1 << 20):
            out.write(block)


def fetch(name: str) -> Path:
    """Return the verified local archive for `name`, downloading and locking it if needed."""
    spec = DATASETS[name]
    archive = CACHE / f"{name}.zip"
    lock = read_lock()
    if not archive.exists():
        print(f"downloading {name} ...", flush=True)
        download(spec["url"], archive)
    digest = sha256_file(archive)
    if name in lock:
        if lock[name]["sha256"] != digest:
            raise SystemExit(f"{name}: SHA-256 {digest} does not match the lock {lock[name]['sha256']}; refusing to use it")
    else:
        lock[name] = {"url": spec["url"], "sha256": digest, "bytes": archive.stat().st_size, "kind": spec["kind"]}
        write_lock(lock)
        print(f"locked {name}: {digest}")
    return archive


def fetch_files(name: str) -> Path:
    """Multi-file Hugging Face datasets: every file is downloaded, hashed and locked individually."""
    spec = DATASETS[name]
    folder = CACHE / name
    lock = read_lock()
    entry = lock.setdefault(name, {"kind": spec["kind"], "repo": spec["repo"], "rev": spec["rev"], "files": {}})
    changed = False
    for file in spec["files"]:
        dest = folder / file
        url = HF.format(repo=spec["repo"], rev=spec["rev"], file=file)
        if not dest.exists():
            print(f"downloading {name}/{file} ...", flush=True)
            download(url, dest)
        digest = sha256_file(dest)
        if file in entry["files"]:
            if entry["files"][file]["sha256"] != digest:
                raise SystemExit(f"{name}/{file}: SHA-256 {digest} does not match the lock; refusing to use it")
        else:
            entry["files"][file] = {"url": url, "sha256": digest, "bytes": dest.stat().st_size}
            changed = True
            print(f"locked {name}/{file}: {digest}")
    if changed:
        write_lock(lock)
    return folder


def load_syntec(name: str) -> tuple[dict, dict, dict]:
    folder = fetch_files(name)
    docs = json.loads((folder / "documents.json").read_text(encoding="utf-8"))
    rows = json.loads((folder / "queries.json").read_text(encoding="utf-8"))
    corpus = {str(d["id"]): (d.get("title", "") or "", d["content"]) for d in docs}  # MTEB: title + text
    queries = {str(i): r["Question"] for i, r in enumerate(rows)}
    qrels = {str(i): {str(r["Article"]): 1} for i, r in enumerate(rows)}
    return corpus, queries, qrels


def load_mteb_parquet(name: str) -> tuple[dict, dict, dict]:
    """BEIR-style parquet (corpus/queries/qrels). Converted once with pyarrow in a throw-away uv environment."""
    import subprocess
    folder = fetch_files(name)
    out = folder / "converted"
    marker = out / ".done"
    if not marker.exists():
        out.mkdir(exist_ok=True)
        subprocess.run(["uv", "run", "--no-project", "--with", "pyarrow", "python", "-I", str(HERE / "convert_parquet.py"),
                        str(folder), str(out)], check=True)
        marker.write_text("ok")
    lock = read_lock()
    lock[name].setdefault("converted_sha256", {})
    for f in ("corpus.jsonl", "queries.jsonl", "qrels.tsv"):
        digest = sha256_file(out / f)
        known = lock[name]["converted_sha256"].get(f)
        if known and known != digest:
            raise SystemExit(f"{name}/{f}: converted file changed ({digest} vs lock {known})")
        if not known:
            lock[name]["converted_sha256"][f] = digest
            write_lock(lock)
    corpus = {}
    for line in (out / "corpus.jsonl").read_text(encoding="utf-8").splitlines():
        d = json.loads(line)
        corpus[str(d["_id"])] = (d.get("title", "") or "", d.get("text", "") or "")
    queries = {str(json.loads(l)["_id"]): json.loads(l)["text"] for l in (out / "queries.jsonl").read_text(encoding="utf-8").splitlines()}
    qrels: dict[str, dict[str, int]] = {}
    with (out / "qrels.tsv").open(encoding="utf-8") as f:
        for row in csv.reader(f, delimiter="	"):
            qrels.setdefault(row[0], {})[row[1]] = int(row[2])
    queries = {q: t for q, t in queries.items() if q in qrels}
    return corpus, queries, qrels


def load_beir(name: str, split: str = "test") -> tuple[dict, dict, dict]:
    archive = fetch(name)
    root = CACHE / name
    marker = root / ".extracted"
    if not marker.exists():
        with zipfile.ZipFile(archive) as z:
            z.extractall(CACHE)
        marker.write_text(sha256_file(archive))
    base = root
    corpus = {}
    for line in (base / "corpus.jsonl").read_text(encoding="utf-8").splitlines():
        d = json.loads(line)
        corpus[str(d["_id"])] = (d.get("title", "") or "", d.get("text", "") or "")
    queries = {}
    for line in (base / "queries.jsonl").read_text(encoding="utf-8").splitlines():
        d = json.loads(line)
        queries[str(d["_id"])] = d["text"]
    qrels: dict[str, dict[str, int]] = {}
    qrels_file = DATASETS[name]["dev_file"] if split == "dev" else DATASETS[name]["split"]
    with (base / "qrels" / f"{qrels_file}.tsv").open(encoding="utf-8") as f:
        for row in csv.reader(f, delimiter="\t"):
            if row[0] == "query-id":
                continue
            qrels.setdefault(row[0], {})[row[1]] = int(row[2])
    queries = {q: t for q, t in queries.items() if q in qrels}  # the split's own queries only
    return corpus, queries, qrels


def load_longmemeval(name: str = "longmemeval_s") -> list[dict]:
    folder = fetch_files(name)
    return json.loads((folder / DATASETS[name]["files"][0]).read_text(encoding="utf-8"))


def is_dev(qid: str) -> bool:
    """Fixed 30 percent tuning split for datasets without an official dev set (stable across runs and machines)."""
    return int(hashlib.sha1(qid.encode()).hexdigest()[:8], 16) % 10 < 3


def load_union(name: str, split: str) -> tuple[dict, dict, dict]:
    corpus, queries, qrels = {}, {}, {}
    for part in DATASETS[name]["parts"]:
        c, q, r = load(part, split)
        corpus.update({f"{part}__{d}": v for d, v in c.items()})
        queries.update({f"{part}__{k}": v for k, v in q.items()})
        qrels.update({f"{part}__{k}": {f"{part}__{d}": rel for d, rel in v.items()} for k, v in r.items()})
    return corpus, queries, qrels


def load(name: str, split: str = "test") -> tuple[dict, dict, dict]:
    """split: test = the official evaluation set (what published anchors use); dev = tuning set; heldout = test minus the tuning
    queries (identical to test where an official dev set exists). Tune on dev, report on heldout, never the reverse."""
    kind = DATASETS[name]["kind"]
    if kind == "union":
        return load_union(name, split)
    if kind == "beir":
        return load_beir(name, split)
    corpus, queries, qrels = _load_flat(name)
    if split == "dev":
        keep = {q for q in queries if is_dev(q)}
    elif split == "heldout":
        keep = {q for q in queries if not is_dev(q)}
    else:
        keep = set(queries)
    return corpus, {q: t for q, t in queries.items() if q in keep}, {q: r for q, r in qrels.items() if q in keep}


def _load_flat(name: str) -> tuple[dict, dict, dict]:
    kind = DATASETS[name]["kind"]
    if kind == "syntec":
        return load_syntec(name)
    if kind == "mteb_parquet":
        return load_mteb_parquet(name)
    if kind == "longmemeval":
        raise SystemExit("longmemeval has per-question haystacks: use run_lme.py (it calls load_longmemeval)")
    raise SystemExit(f"unknown dataset kind {kind}")


if __name__ == "__main__":
    for dataset in sys.argv[1:] or DATASETS:
        if DATASETS[dataset]["kind"] == "longmemeval":
            questions = load_longmemeval(dataset)
            print(f"{dataset}: {len(questions)} questions")
            continue
        corpus, queries, qrels = load(dataset)
        print(f"{dataset}: {len(corpus)} documents, {len(queries)} queries, {sum(len(v) for v in qrels.values())} judgments")


def dataset_pin(name: str) -> dict:
    """What a receipt records to identify the data: hashes of every locked file (recursively for unions)."""
    spec = DATASETS[name]
    if spec["kind"] == "union":
        return {"parts": {p: dataset_pin(p) for p in spec["parts"]}}
    lock = read_lock()[name]
    return {"sha256": lock.get("sha256") or {f: v["sha256"] for f, v in lock["files"].items()},
            "converted_sha256": lock.get("converted_sha256"), "source": lock.get("url") or lock.get("repo")}
