#!/usr/bin/env python3
"""Check that every fact line in docs/reference carries an evidence tag.

A fact line is a list item or a table body row in the two notes documents. Each must contain at least one tag of the form `[rung: evidence]` (or bare `[open]`) where rung is one of the five defined in docs/reference/README.md. Relative links in every reference document must resolve.

Exit status is 1 when anything fails, so the script can run under `make lint-docs`.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REF = os.path.join(ROOT, "docs", "reference")
NOTES = ["nms-save-notes.md", "nms-game-notes.md"]
RUNGS = ("verified", "game-data", "community", "inferred", "open")

TAG = re.compile(r"`\[(" + "|".join(RUNGS) + r")(?::[^\]]*)?\]`")
ANY_TAG = re.compile(r"`\[([a-z-]+)(?::[^\]]*)?\]`")
LINK = re.compile(r"\]\(([^)#]+)(#[^)]*)?\)")
TABLE_SEP = re.compile(r"^\|\s*-+")


def check_tags(path):
    problems = []
    with open(path, encoding="utf-8") as f:
        lines = [line.rstrip("\n") for line in f]
    for i, s in enumerate(lines):
        n = i + 1
        is_bullet = s.startswith("- ")
        is_header = s.startswith("|") and i + 1 < len(lines) and TABLE_SEP.match(lines[i + 1])
        is_row = s.startswith("|") and not TABLE_SEP.match(s) and not is_header
        if not (is_bullet or is_row):
            continue
        for m in ANY_TAG.finditer(s):
            if m.group(1) not in RUNGS:
                problems.append((n, f"unknown rung `{m.group(1)}`"))
        if not TAG.search(s):
            problems.append((n, "no evidence tag"))
    return problems


def check_links(path):
    problems = []
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            for m in LINK.finditer(line):
                target = m.group(1)
                if target.startswith(("http://", "https://")):
                    continue
                full = os.path.normpath(os.path.join(os.path.dirname(path), target))
                if not os.path.exists(full):
                    problems.append((n, f"broken link {target}"))
    return problems


def main():
    failed = False
    for name in sorted(os.listdir(REF)):
        if not name.endswith(".md"):
            continue
        path = os.path.join(REF, name)
        problems = check_links(path)
        if name in NOTES:
            problems += check_tags(path)
        for n, msg in sorted(problems):
            failed = True
            print(f"{os.path.relpath(path, ROOT)}:{n}: {msg}")
    if failed:
        return 1
    print("reference docs: every fact line is tagged, every link resolves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
