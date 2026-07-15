#!/usr/bin/env python3
"""Validate the strict YAML subset used by local Agent Skills."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
KEY_RE = re.compile(r"^[a-z][a-z0-9-]*$")
REF_RE = re.compile(r"(?:`|\()((?:assets|references|scripts)/[^`)\s]+)")
MAX = {"name": 64, "description": 1024}
KNOWN = {"name", "description"}


def scalar(value: str, lineno: int) -> str:
    value = value.strip()
    if not value:
        raise ValueError(f"line {lineno}: scalar value must not be empty")
    if value.startswith('"'):
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError as exc:
            raise ValueError(f"line {lineno}: invalid double-quoted YAML scalar") from exc
        if not isinstance(decoded, str):
            raise ValueError(f"line {lineno}: scalar must be a string")
        return decoded
    if value.startswith("'"):
        if len(value) < 2 or not value.endswith("'"):
            raise ValueError(f"line {lineno}: unterminated single-quoted YAML scalar")
        return value[1:-1].replace("''", "'")
    if ": " in value or " #" in value or value[0] in "{}[]&*!|>@`\"'":
        raise ValueError(f"line {lineno}: unsafe plain YAML scalar; quote the value")
    return value


def frontmatter(path: Path) -> dict[str, str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "---":
        raise ValueError("missing YAML frontmatter")
    try:
        closing = lines.index("---", 1)
    except ValueError as exc:
        raise ValueError("unterminated YAML frontmatter") from exc

    fields: dict[str, str] = {}
    for lineno, line in enumerate(lines[1:closing], start=2):
        if not line.strip():
            continue
        if line[:1].isspace() or "\t" in line:
            raise ValueError(f"line {lineno}: nested or tab-indented frontmatter is not supported")
        key, sep, value = line.partition(":")
        key = key.strip()
        if not sep:
            raise ValueError(f"line {lineno}: expected key: value")
        if not KEY_RE.fullmatch(key):
            raise ValueError(f"line {lineno}: invalid frontmatter key")
        if key in fields:
            raise ValueError(f"line {lineno}: duplicate frontmatter key: {key}")
        fields[key] = scalar(value, lineno)
    return fields


def validate(path: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    try:
        fields = frontmatter(path)
    except ValueError as exc:
        return [str(exc)]

    for key in ("name", "description"):
        if not fields.get(key):
            errors.append(f"missing required field: {key}")

    unknown = sorted(set(fields) - KNOWN)
    if unknown:
        errors.append("unknown frontmatter fields: " + ", ".join(unknown))

    name = fields.get("name", "")
    if name:
        if len(name) > MAX["name"]:
            errors.append("name exceeds 64 characters")
        if not NAME_RE.fullmatch(name):
            errors.append("name must use lowercase letters, numbers, and single hyphens")
        if name != path.parent.name:
            errors.append(f"name must match parent directory ({path.parent.name})")

    description = fields.get("description", "")
    if len(description) > MAX["description"]:
        errors.append("description exceeds 1024 characters")

    for ref in sorted(set(REF_RE.findall(text))):
        if not (path.parent / ref).exists() and not Path(ref).exists():
            errors.append(f"missing referenced file: {ref}")
    return errors


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else Path(".agents/skills")
    files = sorted(root.glob("*/SKILL.md")) if root.is_dir() else [root]
    if not files:
        print(f"no SKILL.md files found under {root}", file=sys.stderr)
        return 2

    failed = False
    for path in files:
        errors = validate(path)
        if errors:
            failed = True
            for error in errors:
                print(f"{path}: {error}", file=sys.stderr)
        else:
            print(f"ok {path}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
