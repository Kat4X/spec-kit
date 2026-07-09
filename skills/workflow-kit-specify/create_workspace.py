#!/usr/bin/env python3
"""Create a Workflow Kit workspace with spec.md from this skill's template."""

import argparse
import re
import shutil
import sys
from datetime import datetime
from pathlib import Path

if sys.version_info < (3, 8):
    raise SystemExit("Workflow Kit requires Python 3.8+")


def slug(value: str) -> str:
    text = re.sub(r"[^a-zA-Z0-9]+", "-", value.strip().lower()).strip("-")
    return text or "change"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Create Workflow Kit workspace.")
    parser.add_argument("change_name", help="2-5 word change name; normalized to kebab-case.")
    parser.add_argument("--root", default=".", help="Repository root. Default: current directory.")
    parser.add_argument("--specs-dir", default="ai/specs", help="Specs dir relative to root. Default: ai/specs.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    specs_dir = Path(args.specs_dir)
    base = specs_dir if specs_dir.is_absolute() else root / specs_dir
    base.mkdir(parents=True, exist_ok=True)

    name = slug(args.change_name)
    timestamp = datetime.now().strftime("%Y.%m.%d_%H:%M")
    workspace = base / f"{timestamp}_{name}"
    suffix = 2
    while workspace.exists():
        workspace = base / f"{timestamp}_{name}-{suffix}"
        suffix += 1

    workspace.mkdir()
    template = Path(__file__).resolve().parent / "template" / "SPEC-TEMPLATE.md"
    shutil.copyfile(template, workspace / "spec.md")
    print(workspace)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
