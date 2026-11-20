"""Crawl, keyword index and embedding time per library for one stack (issue #308); not a quality benchmark.

    python -I bench/experiments/stack_ingest_timing.py <run-dir> <footnote.exe> [--flow <project-folder>]

With `--flow` (issue #307, B run): `docs ingest` of all 8 libraries with the shipped keyword-first flow in an isolated state dir
(<run-dir>/state); see flow() below. Otherwise per library, sequentially, one run: `crawl` (product default page cap), `index` (lexical, FTS only), `rebuild-embeddings` with the
shipped default pair (bekko-a8m + gte-small). Writes <run-dir>/result.json and run.log (timestamps, CPU load sampled before each step).
"""
import json, os, shutil, subprocess, sys, time
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


FLOW_URLS = [u for u in (v[0] for v in STACK.values())]
FLOW_URLS[FLOW_URLS.index("https://docs.pydantic.dev/latest/")] = "https://pydantic.dev/docs/validation/latest/"  # the old docs.pydantic.dev redirect narrows the crawl scope, see STACK-INGEST-TIMING-2026-11-20.md


def flow(project):
    """One `docs ingest` run; times (s since start) when each library's database appears (keyword-searchable: the build renames
    it into place only when indexed) and when `docs list` reports it `ready` (all vectors), plus the first working `find`."""
    env = dict(os.environ, FOOTNOTE_STATE_DIR=str(RUN / "state"))
    host = lambda u: u.split("/")[2]
    db = lambda u: RUN / "state/docs" / host(u) / "latest.db"
    for m in ("bekko-a8m", "gte-small"):  # models live in <state>/models and are selected per state dir
        shutil.copytree(Path(os.environ["LOCALAPPDATA"]) / "footnote/models" / m, RUN / "state/models" / m, dirs_exist_ok=True)
    r = subprocess.run([EXE, "models", "use", "bekko-a8m", "gte-small"], env=env, capture_output=True, text=True)  # per state dir
    assert r.returncode == 0, r.stderr
    say(f"flow: cpu% before {cpu()}")
    t0 = time.time()
    p = subprocess.Popen([EXE, "docs", "ingest", *FLOW_URLS, "--project", project], env=env, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, text=True, encoding="utf-8")
    kw, ready, first_find, fg_return = {}, {}, None, None
    while True:
        now = time.time() - t0
        for u in FLOW_URLS:
            if u not in kw and db(u).is_file():
                kw[u] = now
                say(f"flow: {host(u)} keyword-ready at {now:.1f} s")
        if kw and first_find is None:
            r = subprocess.run([EXE, "find", "request session"], env=env, capture_output=True, text=True, encoding="utf-8")
            if r.returncode == 0 and r.stdout.strip():
                first_find = {"t": round(time.time() - t0, 1), "out": r.stdout.strip()[:300]}
                say(f"flow: first `find` answered at {first_find['t']} s: {first_find['out'][:120]!r}")
        if fg_return is None and p.poll() is not None:
            fg_return = time.time() - t0
            say(f"flow: ingest command returned at {fg_return:.1f} s, exit {p.returncode}")
        if fg_return is not None:
            r = subprocess.run([EXE, "docs", "list"], env=env, capture_output=True, text=True, encoding="utf-8")
            rows = {x["site"]: x for x in json.loads(r.stdout)["docs"]} if r.returncode == 0 else {}
            for u in FLOW_URLS:
                if u not in ready and rows.get(host(u), {}).get("state") == "ready":
                    ready[u] = time.time() - t0
                    say(f"flow: {host(u)} fully embedded at {ready[u]:.1f} s")
            if len(ready) == len(FLOW_URLS) or time.time() - t0 > 3600:
                break
        time.sleep(3 if fg_return is not None else 0.25)
    out, err = p.communicate()
    rows = {x["site"]: x for x in json.loads(subprocess.run([EXE, "docs", "list"], env=env, capture_output=True, text=True,
                                                           encoding="utf-8").stdout)["docs"]}
    res = {"binary": EXE, "project": project, "ingest_stdout": out[-3000:], "ingest_stderr": err[-500:], "foreground_s": fg_return,
           "total_s": time.time() - t0, "first_find": first_find, "libs": {
               host(u): {"pages": rows.get(host(u), {}).get("pages"), "keyword_s": kw.get(u), "embedded_s": ready.get(u)}
               for u in FLOW_URLS}}
    (RUN / "result-flow.json").write_text(json.dumps(res, indent=1), encoding="utf-8")
    say(f"flow: done, total {res['total_s']:.1f} s")


def main():
    RUN.mkdir(parents=True, exist_ok=True)
    if sys.argv[3:4] == ["--flow"]:
        return flow(sys.argv[4])
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
