import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_module(name: str, relative_path: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


scanner = load_module(
    "workflow_list_specs",
    ".agents/skills/workflow-kit/scripts/workflow_list_specs.py",
)
creator = load_module(
    "create_workspace",
    ".agents/skills/workflow-kit/scripts/create_workspace.py",
)
skill_validator = load_module(
    "validate_skills",
    ".agents/skills/workflow-kit/scripts/validate_skills.py",
)


def confirmation(required=False, status="not_required"):
    decided = status in {"approved", "rejected"}
    return {
        "required": required,
        "status": status,
        "request": "Разрешить shared config" if required else None,
        "evidence": "Пользователь подтвердил" if decided else "",
        "updatedAt": "2026-07-09 22:00" if decided else None,
    }


def task(task_id, *, depends_on=None, confirmation_value=None, status="pending"):
    return {
        "id": task_id,
        "title": f"Task {task_id}",
        "details": "Implement the requested behavior.",
        "status": status,
        "priority": "P0",
        "refs": ["R-001"],
        "dependsOn": list(depends_on or []),
        "parallel": False,
        "confirmation": confirmation_value or confirmation(),
        "files": {"read": [], "edit": ["file.txt"], "create": [], "delete": []},
        "checks": [
            {
                "type": "automated",
                "command": "true",
                "scenario": None,
                "required": True,
                "covers": ["R-001"],
            }
        ],
        "verification": {"result": "not_run", "evidence": "", "updatedAt": None},
    }


def tasks_data(tasks, order=None):
    order = order or [item["id"] for item in tasks]
    return {
        "version": 2,
        "change": "fixture",
        "spec": "spec.md",
        "plan": "plan.md",
        "research": None,
        "dataModel": None,
        "scope": "scope.md",
        "verification": "verification.md",
        "tasks": tasks,
        "execution": {"order": order, "firstTask": order[0]},
        "coverage": [],
        "summary": {
            "total": len(tasks),
            "required": len(tasks),
            "parallel": sum(1 for item in tasks if item.get("parallel") is True),
            "confirmationRequired": sum(
                1
                for item in tasks
                if isinstance(item.get("confirmation"), dict)
                and item["confirmation"].get("required") is True
            ),
            "readyForImplement": True,
        },
    }


class WorkspaceFixture:
    def __init__(self, data):
        self.temporary = tempfile.TemporaryDirectory()
        self.path = Path(self.temporary.name) / "2026.07.09_22:00_fixture"
        self.path.mkdir()
        for name in ("spec.md", "plan.md", "scope.md", "verification.md"):
            (self.path / name).write_text(name, encoding="utf-8")
        (self.path / "tasks.json").write_text(
            json.dumps(data, ensure_ascii=False),
            encoding="utf-8",
        )

    def close(self):
        self.temporary.cleanup()


class SkillFrontmatterTests(unittest.TestCase):
    def test_all_local_skills_are_valid(self):
        for path in sorted((ROOT / ".agents/skills").glob("*/SKILL.md")):
            self.assertEqual([], skill_validator.validate(path), path)

    def test_unquoted_colon_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            skill_dir = Path(directory) / "bad-skill"
            skill_dir.mkdir()
            path = skill_dir / "SKILL.md"
            path.write_text(
                "---\nname: bad-skill\ndescription: Invalid: YAML\n---\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "quote the value"):
                skill_validator.frontmatter(path)

    def test_trigger_eval_matrix_routes_all_phases_to_one_skill(self):
        path = ROOT / ".agents/skills/workflow-kit/references/trigger-evals.json"
        cases = json.loads(path.read_text(encoding="utf-8"))
        positive = {case["skill"] for case in cases if case["should_trigger"] is True}
        self.assertEqual({"workflow-kit"}, positive)
        self.assertTrue(any(case["should_trigger"] is False and case["skill"] is None for case in cases))
        for case in cases:
            self.assertIsInstance(case["query"], str)
            self.assertTrue(case["query"].strip())
            self.assertIsInstance(case["should_trigger"], bool)


class ScannerTests(unittest.TestCase):
    def evaluate(self, data):
        fixture = WorkspaceFixture(data)
        self.addCleanup(fixture.close)
        return scanner.evaluate_workspace(fixture.path)

    def test_shipped_tasks_template_matches_schema(self):
        path = ROOT / ".agents/skills/workflow-kit/assets/TASKS-TEMPLATE.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual([], scanner.validate_tasks_data(data))

    def test_missing_id_is_broken(self):
        item = task("T001")
        del item["id"]
        data = tasks_data([item], order=["T001"])
        result = self.evaluate(data)
        self.assertEqual("BROKEN", result["status"])
        self.assertTrue(any("id must match" in problem for problem in result["problems"]))

    def test_duplicate_ids_are_broken(self):
        data = tasks_data([task("T001"), task("T002")])
        data["tasks"][1]["id"] = "T001"
        result = self.evaluate(data)
        self.assertEqual("BROKEN", result["status"])
        self.assertIn("task ids must be unique", result["problems"])

    def test_schema_v1_string_confirmation_flag_is_broken(self):
        item = task("T001")
        item.pop("confirmation")
        item["confirmationRequired"] = "false"
        item["confirmation"] = None
        data = tasks_data([item])
        data["version"] = 1
        result = self.evaluate(data)
        self.assertEqual("BROKEN", result["status"])
        self.assertTrue(any("must be a boolean" in problem for problem in result["problems"]))

    def test_boolean_schema_version_is_broken(self):
        data = tasks_data([task("T001")])
        data["version"] = True
        result = self.evaluate(data)
        self.assertEqual("BROKEN", result["status"])
        self.assertTrue(any("unsupported" in problem for problem in result["problems"]))

    def test_confirmation_lifecycle_controls_readiness(self):
        pending = tasks_data(
            [task("T001", confirmation_value=confirmation(True, "pending"))]
        )
        self.assertEqual("BLOCKED", self.evaluate(pending)["status"])

        approved = tasks_data(
            [task("T001", confirmation_value=confirmation(True, "approved"))]
        )
        self.assertEqual("READY", self.evaluate(approved)["status"])

    def test_execution_order_controls_next_task(self):
        data = tasks_data([task("T001"), task("T002")], order=["T002", "T001"])
        result = self.evaluate(data)
        self.assertEqual(["T002", "T001"], [item["id"] for item in result["readyTasks"]])

    def test_batch_queue_simulates_dependency_chain(self):
        data = tasks_data(
            [
                task("T001"),
                task("T002", depends_on=["T001"]),
                task("T003", depends_on=["T002"]),
            ]
        )
        self.assertEqual([], scanner.validate_tasks_data(data))
        batch = scanner.build_batch_queue(data)
        self.assertEqual(["T001", "T002", "T003"], [item["id"] for item in batch["queue"]])
        self.assertEqual([], batch["blockers"])


class WorkspaceCreatorTests(unittest.TestCase):
    def test_slug_preserves_cyrillic(self):
        self.assertEqual("экспорт-задач", creator.slug("Экспорт задач"))
        self.assertEqual("csv-экспорт-задач", creator.slug("CSV экспорт задач"))


if __name__ == "__main__":
    unittest.main()
