import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("validate_skill", ROOT / "tools/validate_skill.py")
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class SkillValidationTests(unittest.TestCase):
    def test_standard_optional_fields_and_relative_reference(self):
        with tempfile.TemporaryDirectory() as directory:
            skill = Path(directory) / "test-skill"
            (skill / "assets").mkdir(parents=True)
            (skill / "assets/example.txt").write_text("example\n")
            path = skill / "SKILL.md"
            path.write_text(
                """---
name: test-skill
description: "Test skill."
license: MIT
compatibility: "Requires Git."
metadata:
  vendor-key: value
allowed-tools: Bash Read
---

Use `assets/example.txt`.
"""
            )
            self.assertEqual([], validator.validate(path))

    def test_reference_must_exist_under_skill_root(self):
        with tempfile.TemporaryDirectory() as directory:
            skill = Path(directory) / "test-skill"
            skill.mkdir()
            path = skill / "SKILL.md"
            path.write_text(
                "---\nname: test-skill\ndescription: Test.\n---\n\nUse `assets/missing.txt`.\n"
            )
            self.assertEqual(
                ["missing referenced file: assets/missing.txt"], validator.validate(path)
            )


if __name__ == "__main__":
    unittest.main()
