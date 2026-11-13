"""Development comparison (NOT a receipt): the shipped binary on the indexes the receipts built, with a cap on chunks per source file.

    bench/.labenv/Scripts/python.exe -I bench/experiments/dev_per_source.py alloprof syntec

Reuses bench/work/<dataset>/ens/k.db (vectors built by the same helper). Scores doc-level nDCG@10 like bench/run.py.
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import engines  # noqa: E402
import fetch  # noqa: E402
import metrics  # noqa: E402

for ds in sys.argv[1:]:
    corpus, queries, qrels = fetch.load(ds, "test")
    for cap in (None, 1, 2, 3):
        e = engines.ShippedEnsemble()
        e.db = engines.WORK / ds / "ens" / "k.db"
        e.per_source = cap
        rankings = {q: metrics.dedupe(e.search(q, t, 20))[:20] for q, t in queries.items()}
        e.close()
        m = metrics.evaluate(rankings, qrels)
        print(f"{ds:<10} per_source={cap!s:<5} nDCG@10 {m['ndcg@10']:.4f}  R@10 {m['recall@10']:.4f}  queries {m['queries_scored']}", flush=True)
