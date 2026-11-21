#!/usr/bin/env python3
"""office_check: pre-delivery scan of .pptx / .docx / .xlsx files for what should not reach a client.

usage: python office_check.py <files...> [--json]
Exit 1 on any FAIL. Reads the files only; never modifies them.
"""
import json
import re
import sys
import zipfile

RULES = [
    # id, level, regex over the visible text of each part, message
    ("O1-placeholder", "FAIL", re.compile(r"\{\{|\}\}|\bTODO\b|\bTBD\b|\bXXX\b|lorem ipsum|\[(?:insert|placeholder)[^\]]*\]", re.I), "unresolved placeholder"),
    ("O2-local-path", "FAIL", re.compile(r"\b[A-Z]:\\(?:Users|Documents and Settings)\\|/home/[a-z_][\w.-]*/|/Users/[A-Za-z][\w.-]*/"), "personal local path"),
    ("O3-mojibake", "FAIL", re.compile(r"Ã[ -¿]|â€[™œ˜“”’¦]|�"), "broken encoding (mojibake)"),
]
TEXT = re.compile(r"<(?:a|w):t(?:\s[^>]*)?>([^<]*)</(?:a|w):t>|<t(?:\s[^>]*)?>([^<]*)</t>")
PIC = re.compile(r"<p:pic>.*?</p:pic>", re.S)


def check(path):
    out = []
    add = lambda rid, level, part, msg: out.append({"id": rid, "severity": level, "file": path, "part": part, "msg": msg})
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
        for name in names:
            if not name.endswith(".xml"):
                continue
            xml = z.read(name).decode("utf-8", "replace")
            text = " ".join(a or b for a, b in TEXT.findall(xml))
            for rid, level, rx, msg in RULES:
                m = rx.search(text)
                if m:
                    add(rid, level, name, f"{msg}: {text[max(0, m.start() - 30):m.end() + 30]!r}")
            if re.match(r"ppt/slides/slide\d+\.xml$", name):
                if re.search(r'<p:sld\b[^>]*\bshow="0"', xml):
                    add("O5-hidden-slide", "WARN", name, "hidden slide ships with the file")
                for pic in PIC.findall(xml):
                    if not re.search(r'<p:cNvPr\b[^>]*\bdescr="[^"]+"', pic):
                        add("O8-alt-text", "WARN", name, "picture without alt text")
                        break
        comments = [n for n in names if re.search(r"comment", n, re.I) and n.endswith(".xml")]
        if comments:
            add("O4-comments", "FAIL", comments[0], f"{len(comments)} comment part(s): resolve or remove before delivery")
        notes = [n for n in names if re.match(r"ppt/notesSlides/notesSlide\d+\.xml$", n)]
        if notes:
            add("O6-notes", "INFO", notes[0], f"{len(notes)} slides have speaker notes: they ship with the file, read them")
        if "docProps/core.xml" in names:
            core = z.read("docProps/core.xml").decode("utf-8", "replace")
            who = re.findall(r"<(?:dc:creator|cp:lastModifiedBy)>([^<]+)<", core)
            if who:
                add("O7-metadata", "INFO", "docProps/core.xml", f"author metadata: {', '.join(sorted(set(who)))}")
    return out


def main(argv):
    as_json = "--json" in argv
    files = [a for a in argv if a != "--json"]
    if not files:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    found = []
    for f in files:
        try:
            found += check(f)
        except (zipfile.BadZipFile, OSError) as e:
            found.append({"id": "O0-unreadable", "severity": "FAIL", "file": f, "part": "", "msg": str(e)})
    if as_json:
        print(json.dumps(found, indent=2, ensure_ascii=False))
    else:
        for level in ("FAIL", "WARN", "INFO"):
            for x in (x for x in found if x["severity"] == level):
                print(f"{level} {x['id']:<16} {x['file']}:{x['part']}  {x['msg']}")
        fails = sum(x["severity"] == "FAIL" for x in found)
        print(f"{len(found)} findings, {fails} FAIL. Not checked: visual layout, contrast, reading order, embedded media.")
    return 1 if any(x["severity"] == "FAIL" for x in found) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
