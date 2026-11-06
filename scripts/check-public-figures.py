#!/usr/bin/env python3
"""Fail if public text holds a tool count or a version that is out of step.

A count of tools has to match the tool list the reference is generated from, and no
page may type a version: those go out of date with each release, unnoticed. A line
saying what a measure was taken on says so for good, and is let through.
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
COUNTS = ["README.md", "plugin/plugin.json", "plugin/.claude-plugin/plugin.json", "npm/grafta/package.json", "npm/grafta/README.md"]
PAGES = ["README.md", "web/src/content", "web/src/components", "web/src/routes"]
MEASURED_ON = re.compile(r"release build of|measured on", re.IGNORECASE)
VERSION = re.compile(r"(?<![\w.])v?\d+\.\d+\.\d+(?![\w.])")
SKIP = re.compile(r"\.test\.|generated|icons\.")


def pages():
    for name in PAGES:
        path = ROOT / name
        if path.is_file():
            yield path
        else:
            yield from (p for p in path.rglob("*") if p.is_file() and p.suffix in {".ts", ".tsx", ".md"} and not SKIP.search(p.name))


def main():
    tools = json.loads((ROOT / "web/src/generated/tools.json").read_text())["count"]
    wrong = []
    for name in COUNTS:
        path = ROOT / name
        if not path.exists():
            continue
        for line_number, line in enumerate(path.read_text().splitlines(), 1):
            for found in re.findall(r"\b(\d+) tools\b", line):
                if int(found) != tools:
                    wrong.append(f"{name}:{line_number}: says {found} tools, and there are {tools}")
    for path in pages():
        for line_number, line in enumerate(path.read_text().splitlines(), 1):
            if VERSION.search(line) and not MEASURED_ON.search(line):
                wrong.append(f"{path.relative_to(ROOT)}:{line_number}: types a version: {line.strip()[:70]}")
    print("\n".join(wrong) or f"public figures are in step: {tools} tools, no typed version")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
