"""Export experimental capability packs; never install or execute a skill."""

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path

import yaml


PROJECT = Path(__file__).resolve().parent.parent
# Root holding the authored cards (skills/<name>/SKILL.md) and _shared/. They live in the notes vault
# (its AI-Agents folder); point FLOW_SKILL_SOURCES there to rebuild the pack.
AI_ROOT = Path(os.environ.get("FLOW_SKILL_SOURCES") or PROJECT.parent)
NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")
ALLOWED = {"name", "description", "license", "compatibility", "metadata", "allowed-tools"}


def normalize_reference(text):
    text = re.sub(r"\A---\n.*?\n---\n", "", text, count=1, flags=re.S)
    text = "\n".join(line for line in text.splitlines() if not line.startswith(("Up: ", "#domain/", "#skills ")))
    return re.sub(r"\[\[([^]|]+)(?:\|([^]]+))?\]\]", lambda m: m[2] or m[1], text) + "\n"


def read_text(path, root):
    path = Path(path)
    if not path.resolve().is_relative_to(Path(root).resolve()):
        raise ValueError(f"source escapes permitted root: {path.name}")
    with path.open("rb") as stream:
        data = stream.read(1_048_577)
    if len(data) > 1_048_576:
        raise ValueError(f"source exceeds 1 MiB: {path.name}")
    return data.decode("utf-8-sig").replace("\r\n", "\n")


def normalize_skill(text, name):
    match = re.fullmatch(r"---\n(.*?)\n---\n(.*)", text, re.S)
    if not match:
        raise ValueError(f"missing frontmatter: {name}")
    card = yaml.safe_load(match[1])
    if not isinstance(card, dict) or card.get("name") != name:
        raise ValueError(f"skill name mismatch: {name}")
    description = card.get("description")
    if not isinstance(description, str) or not 1 <= len(description.strip()) <= 1024:
        raise ValueError(f"invalid description: {name}")
    exported = {key: value for key, value in card.items() if key in ALLOWED}
    metadata = exported.get("metadata", {})
    if not isinstance(metadata, dict):
        raise ValueError(f"metadata must be a mapping: {name}")
    exported["metadata"] = {str(k): str(v) for k, v in metadata.items()}
    exported["metadata"].update({
        "distribution-status": "experimental; not installed; no measured efficiency claim",
        "source-sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
        "source-card": f"AI-Agents/skills/{name}/SKILL.md",
        "source-normalization": "UTF-8 without BOM; CRLF converted to LF before source hashing",
    })
    body = "\n".join(line for line in match[2].splitlines() if not line.startswith(("Up: ", "#domain/", "#skills ")))
    body = re.sub(r"\[\[([^]|]+)(?:\|([^]]+))?\]\]", lambda m: m[2] or m[1], body)
    return "---\n" + yaml.safe_dump(exported, sort_keys=False, allow_unicode=True) + "---\n" + body + "\n"


def build_pack(pack, output, *, ai_root=AI_ROOT, project=PROJECT):
    ai_root, project, output = Path(ai_root), Path(project), Path(output)
    manifest = json.loads(read_text(project / "capabilities" / "packs.json", project))
    if not isinstance(manifest, dict) or type(manifest.get("schema_version")) is not int or manifest["schema_version"] != 1 or not isinstance(manifest.get("packs"), dict):
        raise ValueError("invalid pack manifest")
    for group in (*manifest["packs"].values(), manifest.get("ui_skills"), manifest.get("measurement_skills"), manifest.get("external_ui_dependencies")):
        if not isinstance(group, list) or any(not isinstance(item, str) for item in group):
            raise ValueError("pack groups and dependencies must be lists of strings")
    if pack == "all":
        names = sorted({name for group in manifest["packs"].values() for name in group})
    else:
        if pack not in manifest["packs"]:
            raise ValueError(f"unknown pack: {pack}")
        names = sorted(set(manifest["packs"][pack]))
    if not names or any(not isinstance(name, str) or not NAME.fullmatch(name) or len(name) > 64 for name in names):
        raise ValueError("invalid skill name in pack")
    resolved_output = output.resolve()
    for protected in (ai_root / "skills", ai_root / "_shared"):
        if resolved_output.is_relative_to(protected.resolve()):
            raise ValueError("output must not be inside source skills or shared protocols")
    if resolved_output.is_relative_to(project.resolve()) and not resolved_output.is_relative_to((project / "build").resolve()):
        raise ValueError("in-project exports are permitted only below project/build")
    # Agent runtimes commonly discover content below a directory named skills.
    # Refuse those roots even when the caller supplies an explicit destination.
    if any(part.casefold() == "skills" for part in resolved_output.parts):
        raise ValueError("output must not be inside a skill installation/discovery root")
    if output.exists():
        raise ValueError("output already exists; choose a new path")

    # Assemble and validate everything before creating any output directory.
    files = {}
    for name in names:
        source = ai_root / "skills" / name / "SKILL.md"
        body = normalize_skill(read_text(source, ai_root / "skills"), name)
        if name in manifest["measurement_skills"]:
            script = "scripts/compare_receipts.py"
            files[f"skills/{name}/{script}"] = read_text(project / "capabilities" / script, project)
            body += "\n## Shipped measurement helper\n\nRun `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.\n"
            files[f"skills/{name}/references/measurement-receipts.md"] = normalize_reference(read_text(project / "capabilities" / "measurement-receipts.md", project))
        files[f"skills/{name}/SKILL.md"] = body
        references = source.parent / "references"
        if references.is_dir():
            for reference in sorted(references.rglob("*.md")):
                relative = reference.relative_to(source.parent).as_posix()
                files[f"skills/{name}/{relative}"] = normalize_reference(read_text(reference, ai_root / "skills"))

    ui = any(name in manifest["ui_skills"] for name in names)
    if ui:
        files["_shared/qloop-ui-protocol.md"] = normalize_reference(read_text(ai_root / "_shared" / "qloop-ui-protocol.md", ai_root / "_shared"))
    if (project / "SOURCE-REVIEW.md").is_file():
        files["_provenance/source-review.md"] = normalize_reference(read_text(project / "SOURCE-REVIEW.md", project))
    missing = []
    for path, body in files.items():
        for target in re.findall(r"\]\((\.\.?/[^)#]+)(?:#[^)]*)?\)", body):
            resolved = (output / path).parent.joinpath(target).resolve()
            if not resolved.is_relative_to(output.resolve()) or resolved.relative_to(output.resolve()).as_posix() not in files:
                missing.append({"from": path, "target": target})
    if missing:
        raise ValueError(f"unbundled relative dependencies: {missing}")
    receipt = {
        "schema_version": 1,
        "pack": pack,
        "status": "experimental",
        "source_review": "_provenance/source-review.md" if "_provenance/source-review.md" in files else None,
        "installed": False,
        "skills": names,
        "external_dependencies": manifest["external_ui_dependencies"] if ui else [],
        "dependency_policy": "External UI prerequisites are not bundled. Leave their acceptance checks pending when unavailable; never install or start agents implicitly.",
        "files": {path: hashlib.sha256(body.encode("utf-8")).hexdigest() for path, body in sorted(files.items())},
    }
    files["pack-manifest.json"] = json.dumps(receipt, indent=2) + "\n"
    output.mkdir(parents=True, exist_ok=False)
    for path, body in files.items():
        destination = output / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(body, encoding="utf-8", newline="\n")
    return receipt


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", help="systems, desktop, intelligence, data, automation, linux, web, all")
    parser.add_argument("output", help="new export directory; not an agent installation path")
    args = parser.parse_args(argv)
    try:
        receipt = build_pack(args.pack, args.output)
    except (OSError, ValueError, RecursionError, yaml.YAMLError) as error:
        print(f"Export failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps({"pack": args.pack, "skills": len(receipt["skills"]), "files": len(receipt["files"]), "installed": False, "external_dependencies": receipt["external_dependencies"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
