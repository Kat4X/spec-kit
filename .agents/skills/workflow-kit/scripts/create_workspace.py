#!/usr/bin/env python3
"""Create a Workflow Kit workspace with spec.md from this skill's template."""

import argparse
import re
import shutil
from datetime import datetime
from pathlib import Path


def slug(value: str) -> str:
    text = "-".join(re.findall(r"[^\W_]+", value.strip().lower(), flags=re.UNICODE))
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
    template = Path(__file__).resolve().parent.parent / "assets" / "SPEC-TEMPLATE.md"
    if not template.is_file():
        raise SystemExit(f"required template not found: {template}")
    base.mkdir(parents=True, exist_ok=True)

    name = slug(args.change_name)
    timestamp = datetime.now().strftime("%Y.%m.%d_%H:%M")
    workspace = base / f"{timestamp}_{name}"
    suffix = 2
    while workspace.exists():
        workspace = base / f"{timestamp}_{name}-{suffix}"
        suffix += 1

    workspace.mkdir()
    target = workspace / "spec.md"
    try:
        shutil.copyfile(template, target)
    except OSError:
        target.unlink(missing_ok=True)
        workspace.rmdir()
        raise
    print(workspace)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
