"""Crawl, keyword index and embedding time per library for one stack (issue #308); not a quality benchmark.

    python -I bench/experiments/stack_ingest_timing.py <run-dir> <footnote.exe>

Per library, sequentially, one run: `crawl` (product default page cap), `index` (lexical, FTS only), `rebuild-embeddings` with the
shipped default pair (bekko-a8m + gte-small). Writes <run-dir>/result.json and run.log (timestamps, CPU load sampled before each step).
"""
import json, os, subprocess, sys, time
from pathlib import Path

RUN, EXE = Path(sys.argv[1]).resolve(), str(Path(sys.argv[2]).resolve())
MODELS = [str(Path(os.environ["LOCALAPPDATA"]) / "footnote/models" / m) for m in ("bekko-a8m", "gte-small")]
STACK = {  # name: (start url, kind)
    "flask": ("https://flask.palletsprojects.com/en/stable/", "Sphinx"),
    "sqlalchemy": ("https://docs.sqlalchemy.org/en/20/", "Sphinx"),
    "requests": ("https://requests.readthedocs.io/en/latest/", "Sphinx"),
    "jinja": ("https://jinja.palletsprojects.com/en/stable/", "Sphinx"),
    "pydantic": ("https://docs.pydantic.dev/latest/", "MkDocs"),
    "vite": ("https://vite.dev/guide/", "VitePress"),
    "serde": ("https://serde.rs/", "mdBook"),
    "tokio": ("https://tokio.rs/tokio/tutorial", "Docusaurus-like"),
}


def say(msg):
    line = f"{time.strftime('%H:%M:%S')} {msg}"
    print(line, flush=True)
    with open(RUN / "run.log", "a", encoding="utf-8") as f:
        f.write(line + "\n")


def cpu():
    r = subprocess.run(["powershell", "-NoProfile", "-Command",
                        r"(Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 3).CounterSamples.CookedValue -join ' '"],
                       capture_output=True, text=True)
    return r.stdout.strip()


def run(*args):
    t = time.time()
    r = subprocess.run([EXE, *args], capture_output=True, text=True, encoding="utf-8", timeout=7200)
    return time.time() - t, r


def main():
    RUN.mkdir(parents=True, exist_ok=True)
    res = {"binary": EXE, "libs": {}}
    only = dict(a.split("=", 1) for a in sys.argv[3:])  # optional second pass: lib=start-url, runs only those libs
    for lib, (url, kind) in STACK.items():
        if only and lib not in only:
            continue
        url = only.get(lib, url)
        pages, db = RUN / "crawl" / lib, RUN / "db" / f"{lib}.db"
        db.parent.mkdir(parents=True, exist_ok=True)
        say(f"{lib}: cpu% before {cpu()}")
        crawl_s, r = run("crawl", url, "--out", str(pages))
        n = len(json.loads((pages / "urls.json").read_text(encoding="utf-8"))) if (pages / "urls.json").is_file() else 0
        say(f"{lib}: crawl {crawl_s:.1f} s, exit {r.returncode}, {n} pages, {r.stdout.strip()[-200:]} {r.stderr.strip()[-200:]}")
        if r.returncode or not n:
            res["libs"][lib] = {"kind": kind, "failed": "crawl"}
            continue
        index_s, r = run("--db", str(db), "index", "--root", str(pages), "--allow", str(pages))
        lex_mb = db.stat().st_size / 1e6
        say(f"{lib}: index {index_s:.1f} s, exit {r.returncode}, db {lex_mb:.1f} MB")
        say(f"{lib}: cpu% before embed {cpu()}")
        embed_s, r = run("--db", str(db), "rebuild-embeddings", *[a for m in MODELS for a in ("--model", m)])
        say(f"{lib}: embed {embed_s:.1f} s, exit {r.returncode}, db {db.stat().st_size / 1e6:.1f} MB {r.stderr.strip()[-200:]}")
        res["libs"][lib] = {"kind": kind, "pages": n, "crawl_s": round(crawl_s, 1), "index_s": round(index_s, 1),
                            "embed_s": round(embed_s, 1), "db_keyword_mb": round(lex_mb, 1),
                            "db_full_mb": round(db.stat().st_size / 1e6, 1), "ok": r.returncode == 0}
        (RUN / "result.json").write_text(json.dumps(res, indent=1), encoding="utf-8")
    say("done")


main()
