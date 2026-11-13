"""Collect published reference scores into anchors.json, with provenance. Run rarely; the output is committed.

Source: the public MTEB results repository (github.com/embeddings-benchmark/results). We record the repository commit, the file
path, and every `test` entry of the task file, so a number can be traced and re-read. BEIR-paper BM25 figures are added by hand
below with the table they come from and are flagged `verified_by_fetch: false` until checked against the paper.
"""
from __future__ import annotations

import base64
import datetime as dt
import json
import subprocess
from pathlib import Path

REPO = "embeddings-benchmark/results"
MODELS = ["sentence-transformers__paraphrase-multilingual-MiniLM-L12-v2", "intfloat__multilingual-e5-small",
          "intfloat__multilingual-e5-base", "BAAI__bge-small-en-v1.5", "thenlper__gte-small",
          "sentence-transformers__all-MiniLM-L6-v2", "hotchpotch__bekko-embedding-v1-a8m",
          "hotchpotch__bekko-embedding-v1-a25m", "ibm-granite__granite-embedding-97m-multilingual-r2",
          "ibm-granite__granite-embedding-small-english-r2", "BAAI__bge-m3",
          "google__embeddinggemma-300m", "microsoft__harrier-oss-v1-270m", "Snowflake__snowflake-arctic-embed-m-v2.0"]
TASKS = ["SciFact", "NFCorpus", "ArguAna", "FiQA2018", "SyntecRetrieval", "AlloprofRetrieval", "BSARDRetrieval",
         "MintakaRetrieval", "XPQARetrieval"]


def gh(path: str) -> dict | list | None:
    out = subprocess.run(["gh", "api", f"repos/{REPO}/{path}"], capture_output=True, text=True)
    return json.loads(out.stdout) if out.returncode == 0 and out.stdout.strip() else None


def main() -> None:
    commit = gh("commits/main")["sha"]
    anchors: dict = {"retrieved_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
                     "mteb_results_commit": commit, "mteb": [], "beir_bm25": []}
    for model in MODELS:
        for rev in gh(f"contents/results/{model}") or []:
            files = {f["name"] for f in gh(f"contents/results/{model}/{rev['name']}") or []}
            for task in TASKS:
                if f"{task}.json" not in files:
                    continue
                path = f"results/{model}/{rev['name']}/{task}.json"
                blob = gh(f"contents/{path}")
                data = json.loads(base64.b64decode(blob["content"]))
                entries = [
                    {"subset": e.get("hf_subset"), "languages": e.get("languages"),
                     "ndcg_at_10": e.get("ndcg_at_10"), "recall_at_10": e.get("recall_at_10"),
                     "main_score": e.get("main_score")}
                    for e in data.get("scores", {}).get("test", [])
                ]
                anchors["mteb"].append({"model": model, "model_revision": rev["name"], "task": task, "path": path,
                                        "mteb_version": data.get("mteb_version"), "scores_test": entries})
    # BEIR paper (Thakur et al., 2021), BM25 nDCG@10 on the test split. Typed from the paper's results table; NOT yet re-fetched.
    for name, value in (("scifact", 0.665), ("nfcorpus", 0.325)):
        anchors["beir_bm25"].append({"dataset": name, "ndcg_at_10": value, "source": "BEIR paper, BM25 row (Elasticsearch)",
                                     "url": "https://arxiv.org/abs/2104.08663", "verified_by_fetch": False})
    Path(__file__).with_name("anchors.json").write_text(json.dumps(anchors, indent=2) + "\n", encoding="utf-8")
    print(f"commit {commit[:10]}; {len(anchors['mteb'])} task files recorded")
    for a in anchors["mteb"]:
        s = a["scores_test"][0] if a["scores_test"] else {}
        print(f"  {a['model'].split('__')[1][:34]:<34} {a['model_revision'][:8]} {a['task']:<18} subsets={len(a['scores_test'])} "
              f"ndcg@10={s.get('ndcg_at_10')} ({s.get('subset')})")


if __name__ == "__main__":
    main()
