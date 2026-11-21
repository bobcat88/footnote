"""Skill ranking eval (#279): can Footnote pick the one skill that matches a prompt, and stay silent when none does?

Each skill becomes one Library document (name + description). For corpus sizes 10/30/100/all (the labelled target skills
plus seeded random distractors) every labelled prompt is searched lexically and in hybrid mode, and three injection gates
are scored:
  - coverage>=c: the top hit holds at least c of the prompt's topic words (Footnote's fetch-on-miss rule, miss.rs)
  - agree: lexical and hybrid rank the same skill first
  - always: inject the top hit (the no-gate baseline)
Metrics: top-1 / top-3 over prompts with a target; for each gate, correct injections (target prompts where the gate fires
on the right skill) and false injections (gate fires on a "none" prompt, or on the wrong skill).

The corpus and labels are local (third-party skill texts stay out of the repo):
  python bench/experiments/skill_ranking.py --skills ~/.claude/skills ~/.claude/plugins \
      --labels artifacts/skill-ranking/labels.jsonl --out artifacts/skill-ranking --model <models>/bekko-a8m --model <models>/gte-small
"""
import argparse, json, os, random, re, shutil, subprocess, sys
from pathlib import Path

STOP = {"the", "and", "for", "how", "what", "does", "with", "that", "this", "from", "can", "are", "use", "using", "when",
        "which", "why", "its", "into", "not", "you", "your", "get", "set"}


def terms(q):  # same rule as footnote_library::miss::terms
    seen, out = set(), []
    for w in re.split(r"[^\w]|_", q.lower()):
        if len(w) >= 3 and w not in STOP and w not in seen:
            seen.add(w)
            out.append(w)
    return out


def coverage(q, text):
    want = terms(q)
    return sum(t in text.lower() for t in want) / len(want) if want else 0.0


def skills(roots):
    found = {}
    for root in roots:
        for f in Path(root).expanduser().rglob("SKILL.md"):
            text = f.read_text(encoding="utf-8", errors="replace")
            m = re.match(r"---\s*\n(.*?)\n---", text, re.S)
            if not m:
                continue
            front = m.group(1)
            name = re.search(r"^name:\s*['\"]?(.+?)['\"]?\s*$", front, re.M)
            desc = re.search(r"^description:\s*(.+?)(?=^\w[\w-]*:|\Z)", front, re.M | re.S)
            if name and desc and name.group(1) not in found:
                d = " ".join(desc.group(1).replace(">-", " ").replace("|", " ").split()).strip("'\"")
                found[name.group(1)] = d[:1500]
    return found


def footnote(state, *args):
    env = dict(os.environ, FOOTNOTE_STATE_DIR=str(state), FOOTNOTE_NO_LIVE_FETCH="1")
    out = subprocess.run(["footnote", *args], capture_output=True, text=True, encoding="utf-8", env=env)
    if out.returncode:
        sys.exit(f"footnote {' '.join(args)} failed: {out.stderr}{out.stdout}")
    return json.loads(out.stdout)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--skills", nargs="+", required=True)
    ap.add_argument("--labels", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--model", action="append", default=[])
    ap.add_argument("--sizes", default="10,30,100,0")
    a = ap.parse_args()
    corpus = skills(a.skills)
    labels = [json.loads(l) for l in open(a.labels, encoding="utf-8") if l.strip()]
    targets = sorted({l["skill"] for l in labels if l["skill"] != "none"})
    missing = [t for t in targets if t not in corpus]
    if missing:
        sys.exit(f"target skills not found: {missing}")
    others = sorted(set(corpus) - set(targets))
    random.Random(0).shuffle(others)
    out = Path(a.out)
    rows = []
    for size in [int(s) for s in a.sizes.split(",")]:
        n = len(corpus) if size == 0 else size
        names = targets + others[: max(0, n - len(targets))]
        root = out / f"corpus-{n}"
        docs = root / "skills"
        docs.mkdir(parents=True, exist_ok=True)
        for old in docs.glob("*.md"):
            old.unlink()
        for name in names:
            (docs / f"{name}.md").write_text(f"# {name}\n\n{corpus[name]}\n", encoding="utf-8")
        db, state = root / "skills.db", root / "state"
        for p in root.glob("skills.db*"):
            shutil.rmtree(p) if p.is_dir() else p.unlink()
        footnote(state, "--db", str(db), "index", "--root", str(docs), "--allow", str(docs))
        modes = ["lexical"]
        if a.model:
            footnote(state, "--db", str(db), "rebuild-embeddings", *[x for m in a.model for x in ("--model", m)])
            modes.append("hybrid")
        ranked = {}
        for mode in modes:
            for l in labels:
                extra = [x for m in a.model for x in ("--model", m)] if mode == "hybrid" else []
                hits = footnote(state, "--db", str(db), "search", l["prompt"], "--mode", mode, "--limit", "3", *extra)["results"]
                ranked[(mode, l["prompt"])] = [(Path(h["path"]).stem, h.get("text", "")) for h in hits]
        for mode in modes:
            gates = {f"coverage>={c}": c for c in (0.2, 0.3, 0.4, 0.5, 0.6)}
            gates["always"] = 0.0
            if "hybrid" in modes:
                gates["agree"] = None
            for gate, c in gates.items():
                top1 = top3 = right = wrong = none_fired = 0
                for l in labels:
                    hits = ranked[(mode, l["prompt"])]
                    first = hits[0][0] if hits else None
                    if gate == "agree":
                        other = ranked[("hybrid" if mode == "lexical" else "lexical", l["prompt"])]
                        fires = bool(hits) and bool(other) and other[0][0] == first
                    else:
                        fires = bool(hits) and coverage(l["prompt"], hits[0][1]) >= c
                    if l["skill"] == "none":
                        none_fired += fires
                        continue
                    top1 += first == l["skill"]
                    top3 += l["skill"] in [h[0] for h in hits]
                    if fires:
                        right += first == l["skill"]
                        wrong += first != l["skill"]
                pos = sum(l["skill"] != "none" for l in labels)
                neg = len(labels) - pos
                rows.append({"size": n, "mode": mode, "gate": gate, "top1": top1 / pos, "top3": top3 / pos,
                             "correct_injection": right / pos, "wrong_skill": wrong / pos,
                             "false_on_none": none_fired / neg})
    (out / "results.json").write_text(json.dumps(rows, indent=1))
    print("| skills | mode | gate | top-1 | top-3 | injected right | wrong skill | fired on none |")
    print("|---:|---|---|---:|---:|---:|---:|---:|")
    for r in rows:
        print(f"| {r['size']} | {r['mode']} | {r['gate']} | {r['top1']:.0%} | {r['top3']:.0%} | "
              f"{r['correct_injection']:.0%} | {r['wrong_skill']:.0%} | {r['false_on_none']:.0%} |")


if __name__ == "__main__":
    main()
