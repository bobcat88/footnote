"""Refresh the Unreleased section of CHANGELOG.md from Conventional Commits (git-cliff, config in cliff.toml).

    python scripts/changelog.py [path to git-cliff]

The section is replaced in place, below the file's introduction; released sections are left as they are.
"""
import re
import subprocess
import sys
from pathlib import Path

cliff = str(Path(sys.argv[1]).resolve()) if len(sys.argv) > 1 else "git-cliff"  # Windows needs a full path
section = subprocess.run([cliff, "--unreleased"], capture_output=True, text=True, encoding="utf-8", check=True).stdout.strip()
path = Path("CHANGELOG.md")
text = path.read_text(encoding="utf-8")
text = re.sub(r"\n## Unreleased\n.*?(?=\n## |\Z)", "\n", text, count=1, flags=re.S)  # drop the old Unreleased section
head, sep, rest = text.partition("\n## ")
path.write_text(f"{head.rstrip()}\n\n{section}\n\n{sep.lstrip()}{rest}" if sep else f"{head.rstrip()}\n\n{section}\n",
                encoding="utf-8")
print("CHANGELOG.md: Unreleased section refreshed")
