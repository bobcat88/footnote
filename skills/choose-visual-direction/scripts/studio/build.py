"""Build a Design Studio page from a config JSON.

usage: python -I build.py <config.json> <out.html>

The config names the project, the preview kind ("app" or "page"), optional preview copy, and 2 to 4
directions, each with an id, a name, a one-line description, the references it came from, a screenshot
path (relative to the config file) and its starting tokens. Images are embedded as data URIs (shrunk with ffmpeg when large), so the
published page is one self-contained file.
"""
import base64, json, pathlib, re, shutil, subprocess, sys, tempfile

HERE = pathlib.Path(__file__).parent
TOKEN_KEYS = {"theme", "accent", "warmth", "tint", "radius", "density", "scale", "display", "dweight", "body", "motion"}
FONTS = {"Hanken Grotesk", "Geist", "Instrument Sans", "Manrope", "Figtree", "Plus Jakarta Sans", "Sora", "Bricolage Grotesque",
         "DM Sans", "Outfit", "Montserrat", "Fraunces", "Newsreader", "Source Serif 4", "JetBrains Mono"}


def check_tokens(d):
    t = d.get("tokens", {})
    errs = []
    missing = TOKEN_KEYS - set(t)
    if missing: errs.append(f"missing {sorted(missing)}")
    if t.get("theme") not in ("light", "dark"): errs.append("theme must be light or dark")
    if not re.fullmatch(r"#[0-9a-fA-F]{6}", str(t.get("accent", ""))): errs.append("accent must be #rrggbb")
    for k, lo, hi in (("warmth", -1, 1), ("tint", 0, 6), ("radius", 0, 28), ("density", 0.8, 1.3), ("scale", 1.125, 1.5), ("dweight", 100, 900)):
        if not isinstance(t.get(k), (int, float)) or not lo <= t[k] <= hi: errs.append(f"{k} must be a number in [{lo}, {hi}]")
    for k in ("display", "body"):
        if t.get(k) not in FONTS: errs.append(f"{k} must be one of the studio fonts")
    if t.get("motion") not in ("none", "subtle", "lively"): errs.append("motion must be none, subtle or lively")
    return [f"{d.get('id', '?')}: {e}" for e in errs]


def embed(path, base):
    p = base / path  # relative paths resolve from the config's folder
    if not p.exists(): sys.exit(f"image not found: {p}")
    data, mime = p.read_bytes(), "image/png" if p.suffix.lower() == ".png" else "image/jpeg"
    if len(data) > 350_000 and shutil.which("ffmpeg"):
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "small.jpg"
            subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(p), "-vf", "scale=1280:-2", "-q:v", "5", str(out)], check=True)
            data, mime = out.read_bytes(), "image/jpeg"
    return f"data:{mime};base64," + base64.b64encode(data).decode()


def main():
    if len(sys.argv) != 3: sys.exit(__doc__)
    src = pathlib.Path(sys.argv[1])
    cfg = json.loads(src.read_text(encoding="utf-8"))
    dirs = cfg.get("directions", [])
    if not 2 <= len(dirs) <= 4: sys.exit("config needs 2 to 4 directions")
    errs = [e for d in dirs for e in check_tokens(d)]
    if errs: sys.exit("invalid tokens:\n  " + "\n  ".join(errs))
    if cfg.get("kind") not in ("app", "page"): sys.exit('kind must be "app" or "page"')
    for d in dirs:
        if d.get("image"): d["image"] = embed(d["image"], src.parent)
    page = (HERE / "studio.template.html").read_text(encoding="utf-8")
    for marker in ("/*CONFIG*/{}", "/*TITLE*/Design Studio"):
        if marker not in page: sys.exit(f"template marker missing: {marker}")
    title = (cfg.get("project") or "Design") + " Studio"
    page = page.replace("/*TITLE*/Design Studio", title).replace("/*CONFIG*/{}", json.dumps(cfg))
    out = pathlib.Path(sys.argv[2])
    out.write_text(page, encoding="utf-8")
    print(f"{out} written: {len(dirs)} directions, {len(page) // 1024} KB")


if __name__ == "__main__":
    main()
