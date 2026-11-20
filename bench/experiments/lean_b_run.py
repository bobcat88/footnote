"""Run the Lean B arms on ladder L1/L2 with `claude -p` (#239). From flow/: python ../bench/experiments/lean_b_run.py OUT.jsonl MODEL EFFORT RUNS [arm,...]
Clean profile (no user settings, hooks, MCP, CLAUDE.md); ponytail arm adds only the installed plugin; lean arms append rule sections."""
import json, os, pathlib, random, subprocess, sys, tempfile, time
sys.path.insert(0, os.getcwd())
os.environ.setdefault("FOOTNOTE_BIN", os.path.expanduser("~/AppData/Local/Programs/footnote/footnote.exe"))
from evaluation import luna_pilot, dependency_pilot
FN = os.environ["FOOTNOTE_BIN"]
RULES = pathlib.Path(__file__).resolve().parents[2] / "rules/lean/LEAN-B.md"
TASKS = {"L1": (luna_pilot, ["-m", "integration.check_intervals", "{w}", "--followup"], "TASK.txt, CONTRACT.txt and DECISION.txt"),
         "L2": (dependency_pilot, ["-m", "evaluation.dependency_fixture", "{w}"], "TASK.txt, CONTRACT.txt and DECISION.txt")}

def rule_file(arm, tmp):
    lines = RULES.read_text(encoding="utf-8").splitlines()
    cut = next(i for i, l in enumerate(lines) if l.startswith("## 4.")) if arm == "lean-b-code" else len(lines)
    p = tmp / f"{arm}.md"; p.write_text("\n".join(lines[:cut]), encoding="utf-8"); return p

def workspace(task, tmp):
    mod = TASKS[task][0]; d = tmp / f"ws-{task}-{time.time_ns()}"
    kw = {"arms": ("baseline",), "folders": {"baseline": d / "b"}} if task == "L1" else {}
    mod.run(d, controls=False, **kw)
    return next(d.glob("**/workspace"))

def run_one(arm, task, n, model, effort, tmp):
    work = workspace(task, tmp)
    prompt = (f"Complete exactly one evaluation delivery in {work.as_posix()}. Read {TASKS[task][2]}. Own only files listed in TASK.txt. "
              "No MCP, network, installs, commits or delegation. Run every command from the workspace. Run local tests and deliver once. "
              + ("Include the word checks in delivery.txt. " if task == "L2" else "") + "Short final answer.")
    args = ["claude", "-p", prompt, "--model", model, "--effort", effort, "--setting-sources", "project,local", "--strict-mcp-config",
            "--disable-slash-commands", "--permission-mode", "bypassPermissions", "--output-format", "stream-json", "--verbose"]
    if arm == "ponytail":
        s = tmp / "pt.json"; s.write_text(json.dumps({"enabledPlugins": {"ponytail@ponytail": True}})); args += ["--settings", str(s)]
    elif arm != "bare":
        args += ["--append-system-prompt-file", str(rule_file(arm, tmp))]
    env = {k: v for k, v in os.environ.items() if not k.startswith(("FOOTNOTE_", "CLAUDECODE", "CLAUDE_CODE_"))}
    env["CLAUDE_CODE_DISABLE_CLAUDE_MDS"] = "1"
    t0 = time.time()
    out = subprocess.run(args, cwd=work, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, encoding="utf-8", timeout=1200).stdout
    sec = round(time.time() - t0)
    ev = [json.loads(l) for l in out.splitlines() if l.startswith("{")]
    init = next((e for e in ev if e.get("subtype") == "init"), {}); res = next((e for e in ev if e.get("type") == "result"), {})
    plug = [p["source"] for p in init.get("plugins", []) if not p["source"].endswith("@builtin")]
    active = any("ponytail" in p for p in plug) if arm == "ponytail" else not plug  # rule arms: proven by the flag, plugins must be empty
    sid = res.get("session_id")
    f = next(pathlib.Path.home().glob(f".claude/projects/*/{sid}.jsonl"), None)
    u = json.loads(subprocess.run([FN, "usage", "--file", str(f)], capture_output=True, text=True).stdout)["totals"]
    chk = subprocess.run([sys.executable, *[a.replace("{w}", str(work)) for a in TASKS[task][1]]], capture_output=True, text=True)
    return {"arm": arm, "task": task, "run": n, "accepted": chk.returncode == 0 and active, "turns": res.get("num_turns"),
            "uncached_in": u["input"], "cache_write": u["cache_write_5m"] + u["cache_write_1h"], "cache_read": u["cache_read"],
            "output": u["output"], "seconds": sec, "usd": u.get("api_price_usd"), "plugins": plug, "active": active}

if __name__ == "__main__":
    out, model, effort, runs = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
    arms = sys.argv[5].split(",") if len(sys.argv) > 5 else ["bare", "ponytail", "lean-b-code", "lean-b-full"]
    tmp = pathlib.Path(tempfile.mkdtemp()); rng = random.Random(239)
    for n in range(int(os.environ.get('START', 1)), runs + 1):
        for task in os.environ.get('ONLY', ','.join(TASKS)).split(','):
            order = arms[:]; rng.shuffle(order)
            for arm in order:
                r = run_one(arm, task, n, model, effort, tmp)
                open(out, "a", encoding="utf-8").write(json.dumps(r) + "\n"); print(r, flush=True)
