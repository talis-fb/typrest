#!/usr/bin/env python3
"""Check relative Markdown links and heading anchors.

Usage: scripts/check-doc-links.py [file-or-directory ...]

With no arguments, checks every tracked Markdown file in the repository. External
(scheme-qualified) links are ignored. Exits with status 1 when a link is broken.
"""

import re
import subprocess
import sys
from pathlib import Path

LINK = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def slugify(heading: str) -> str:
    text = re.sub(r"`", "", heading)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def anchors(path: Path) -> set:
    found, counts, in_fence = set(), {}, False
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        match = None if in_fence else HEADING.match(line)
        if match:
            slug = slugify(match.group(2))
            n = counts.get(slug, 0)
            counts[slug] = n + 1
            found.add(slug if n == 0 else f"{slug}-{n}")
    return found


def markdown_files(root: Path, args: list) -> list:
    if not args:
        out = subprocess.run(
            ["git", "ls-files", "*.md"], cwd=root, capture_output=True, text=True, check=True
        ).stdout.split()
        return [root / p for p in out if "node_modules" not in p and (root / p).exists()]
    files = []
    for arg in args:
        p = Path(arg)
        files.extend(sorted(p.rglob("*.md")) if p.is_dir() else [p])
    return files


def main() -> int:
    root = Path(subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip())
    errors, cache = [], {}
    for md in markdown_files(root, sys.argv[1:]):
        in_fence = False
        for number, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            if FENCE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for target in LINK.findall(re.sub(r"`[^`]*`", "", line)):
                if SCHEME.match(target):
                    continue
                path_part, _, fragment = target.partition("#")
                dest = md if not path_part else (md.parent / path_part).resolve()
                if not dest.exists():
                    errors.append(f"{md.relative_to(root)}:{number}: missing file {target}")
                    continue
                if fragment and dest.is_file() and dest.suffix == ".md":
                    if dest not in cache:
                        cache[dest] = anchors(dest)
                    if fragment.lower() not in cache[dest]:
                        errors.append(f"{md.relative_to(root)}:{number}: missing anchor {target}")
    for error in errors:
        print(error)
    print(f"{len(errors)} broken link(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
