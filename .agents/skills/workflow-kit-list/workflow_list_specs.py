#!/usr/bin/env python3
"""List Workflow Kit workspaces ready for implementation.

The scanner is intentionally dependency-free: it only reads ai/specs/* and
computes readiness from tasks.json and required workflow artifacts.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

DONE_STATUSES = {"done", "skipped"}
KNOWN_STATUSES = {"pending", "in_progress", "done", "blocked", "skipped"}
GROUP_ORDER = [
    "READY",
    "BLOCKED",
    "NEEDS_TASKS",
    "NEEDS_PLAN",
    "LEGACY",
    "BROKEN",
    "DONE",
]
DEFAULT_VISIBLE = {status for status in GROUP_ORDER if status != "DONE"}
REQUIRED_FOR_IMPLEMENT = ["spec.md", "plan.md", "scope.md", "tasks.json", "verification.md"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="List Workflow Kit specs/workspaces and runnable tasks.",
    )
    parser.add_argument(
        "--root",
        default=".",
        help="Repository root. Default: current directory.",
    )
    parser.add_argument(
        "--specs-dir",
        default="ai/specs",
        help="Specs directory relative to root, or absolute path. Default: ai/specs.",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Include DONE workspaces in text/JSON output.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        dest="as_json",
        help="Print machine-readable JSON.",
    )
    parser.add_argument(
        "--status",
        action="append",
        choices=GROUP_ORDER,
        help="Filter by workspace status. Can be repeated.",
    )
    parser.add_argument(
        "--max-next",
        type=int,
        default=5,
        help="Maximum ready/blocked tasks shown per workspace in text output. Default: 5.",
    )
    parser.add_argument(
        "--next-task",
        nargs="?",
        const="",
        metavar="WORKSPACE",
        help="Print only the first runnable task as JSON. WORKSPACE can be a path or workspace name; omitted means first READY workspace.",
    )
    return parser.parse_args()


def as_bool(value: Any) -> bool:
    return value is True


def clean_string(value: Any, fallback: str = "") -> str:
    if value is None:
        return fallback
    text = str(value).strip()
    return text if text else fallback


def task_summary(task: Dict[str, Any], reason: Optional[str] = None) -> Dict[str, Any]:
    return {
        "id": clean_string(task.get("id"), "<no-id>"),
        "priority": clean_string(task.get("priority")),
        "title": clean_string(task.get("title"), "<no title>"),
        "status": clean_string(task.get("status"), "<missing>"),
        "reason": reason or "",
    }


def dependency_list(task: Dict[str, Any]) -> Tuple[List[str], List[str]]:
    raw_deps = task.get("dependsOn", [])
    if raw_deps is None:
        return [], []
    if not isinstance(raw_deps, list):
        return [], ["dependsOn is not a list"]
    return [clean_string(dep) for dep in raw_deps if clean_string(dep)], []


def initial_result(workspace: Path) -> Dict[str, Any]:
    return {
        "name": workspace.name,
        "path": str(workspace),
        "status": "BROKEN",
        "progress": {
            "done": 0,
            "total": 0,
            "pending": 0,
            "blocked": 0,
            "inProgress": 0,
            "skipped": 0,
        },
        "readyTasks": [],
        "blockedTasks": [],
        "missingFiles": [],
        "problems": [],
        "command": None,
    }


def evaluate_workspace(workspace: Path) -> Dict[str, Any]:
    result = initial_result(workspace)

    has_spec = (workspace / "spec.md").is_file()
    has_plan = (workspace / "plan.md").is_file()
    has_scope = (workspace / "scope.md").is_file()
    has_tasks_json = (workspace / "tasks.json").is_file()
    has_tasks_md = (workspace / "tasks.md").is_file()

    if not has_spec:
        result["status"] = "BROKEN"
        result["missingFiles"].append("spec.md")
        result["problems"].append("workspace has no spec.md")
        return result

    if not has_tasks_json:
        if has_tasks_md:
            result["status"] = "LEGACY"
            result["missingFiles"].append("tasks.json")
            result["problems"].append("legacy tasks.md exists, but tasks.json is required for implementation")
            return result
        if not has_plan or not has_scope:
            result["status"] = "NEEDS_PLAN"
            if not has_plan:
                result["missingFiles"].append("plan.md")
            if not has_scope:
                result["missingFiles"].append("scope.md")
            result["problems"].append("plan/scope stage is not complete")
            return result
        result["status"] = "NEEDS_TASKS"
        result["missingFiles"].append("tasks.json")
        result["problems"].append("tasks.json is missing")
        return result

    missing_required = [name for name in REQUIRED_FOR_IMPLEMENT if not (workspace / name).is_file()]
    if missing_required:
        result["status"] = "BROKEN"
        result["missingFiles"].extend(missing_required)
        result["problems"].append("implementation-required workflow files are missing")
        return result

    tasks_path = workspace / "tasks.json"
    try:
        with tasks_path.open("r", encoding="utf-8") as file:
            tasks_data = json.load(file)
    except json.JSONDecodeError as error:
        result["status"] = "BROKEN"
        result["problems"].append(f"tasks.json is invalid JSON: {error.msg} at line {error.lineno}, column {error.colno}")
        return result
    except OSError as error:
        result["status"] = "BROKEN"
        result["problems"].append(f"tasks.json cannot be read: {error}")
        return result

    tasks = tasks_data.get("tasks") if isinstance(tasks_data, dict) else None
    if not isinstance(tasks, list):
        result["status"] = "BROKEN"
        result["problems"].append("tasks.json must contain a top-level tasks array")
        return result
    if not tasks:
        result["status"] = "BROKEN"
        result["problems"].append("tasks array is empty")
        return result

    task_ids = {clean_string(task.get("id")) for task in tasks if isinstance(task, dict)}
    task_ids.discard("")
    done_ids = {
        clean_string(task.get("id"))
        for task in tasks
        if isinstance(task, dict) and clean_string(task.get("status")) in DONE_STATUSES
    }
    done_ids.discard("")

    ready_tasks: List[Dict[str, Any]] = []
    blocked_tasks: List[Dict[str, Any]] = []
    status_counts = {"done": 0, "pending": 0, "blocked": 0, "in_progress": 0, "skipped": 0}
    unfinished_count = 0

    for index, raw_task in enumerate(tasks, start=1):
        if not isinstance(raw_task, dict):
            unfinished_count += 1
            blocked_tasks.append(
                {
                    "id": f"<task-{index}>",
                    "priority": "",
                    "title": "Invalid task entry",
                    "status": "<invalid>",
                    "reason": "task entry is not an object",
                }
            )
            continue

        status = clean_string(raw_task.get("status"), "<missing>")
        if status in status_counts:
            status_counts[status] += 1
        if status not in DONE_STATUSES:
            unfinished_count += 1

        if status == "pending":
            deps, dep_errors = dependency_list(raw_task)
            unknown_deps = [dep for dep in deps if dep not in task_ids]
            unmet_deps = [dep for dep in deps if dep not in done_ids]
            reasons: List[str] = []
            reasons.extend(dep_errors)
            if unknown_deps:
                reasons.append("unknown deps: " + ", ".join(unknown_deps))
            elif unmet_deps:
                reasons.append("unmet deps: " + ", ".join(unmet_deps))
            if as_bool(raw_task.get("confirmationRequired")):
                reasons.append("confirmation required")

            if reasons:
                blocked_tasks.append(task_summary(raw_task, "; ".join(reasons)))
            else:
                ready_tasks.append(task_summary(raw_task))
        elif status in {"blocked", "in_progress"}:
            blocked_tasks.append(task_summary(raw_task, f"status is {status}"))
        elif status not in KNOWN_STATUSES:
            blocked_tasks.append(task_summary(raw_task, f"unknown status: {status}"))

    result["progress"] = {
        "done": status_counts["done"] + status_counts["skipped"],
        "total": len(tasks),
        "pending": status_counts["pending"],
        "blocked": status_counts["blocked"],
        "inProgress": status_counts["in_progress"],
        "skipped": status_counts["skipped"],
    }
    result["readyTasks"] = ready_tasks
    result["blockedTasks"] = blocked_tasks

    if ready_tasks:
        result["status"] = "READY"
        first_task = ready_tasks[0]
        result["command"] = f"workflow-kit-implement {workspace.name} {first_task['id']}"
    elif unfinished_count > 0:
        result["status"] = "BLOCKED"
    else:
        result["status"] = "DONE"

    return result


def specs_path(root: Path, specs_dir: str) -> Path:
    candidate = Path(specs_dir)
    if candidate.is_absolute():
        return candidate
    return root / candidate


def resolve_workspace(root: Path, base_dir: Path, value: str) -> Path:
    candidate = Path(value)
    if candidate.is_absolute() or candidate.parts:
        path = candidate if candidate.is_absolute() else root / candidate
        if path.is_dir():
            return path
    return base_dir / value


def load_first_ready_task(workspace: Path, ready_id: str) -> Optional[Dict[str, Any]]:
    try:
        with (workspace / "tasks.json").open("r", encoding="utf-8") as file:
            tasks_data = json.load(file)
    except (OSError, json.JSONDecodeError):
        return None
    tasks = tasks_data.get("tasks") if isinstance(tasks_data, dict) else None
    if not isinstance(tasks, list):
        return None
    for task in tasks:
        if isinstance(task, dict) and clean_string(task.get("id")) == ready_id:
            return task
    return None


def discover_workspaces(base_dir: Path) -> List[Path]:
    if not base_dir.is_dir():
        return []
    return sorted(
        [path for path in base_dir.iterdir() if path.is_dir()],
        key=lambda path: path.name,
        reverse=True,
    )


def status_index(status: str) -> int:
    try:
        return GROUP_ORDER.index(status)
    except ValueError:
        return len(GROUP_ORDER)


def filtered_results(results: List[Dict[str, Any]], args: argparse.Namespace) -> List[Dict[str, Any]]:
    statuses = set(args.status or [])
    if statuses:
        return [item for item in results if item["status"] in statuses]
    if args.all:
        return results
    return [item for item in results if item["status"] in DEFAULT_VISIBLE]


def print_task_list(title: str, tasks: List[Dict[str, Any]], max_next: int) -> None:
    if not tasks:
        return
    print(f"  {title}:")
    for task in tasks[:max_next]:
        priority = f" {task['priority']}" if task.get("priority") else ""
        reason = f" [{task['reason']}]" if task.get("reason") else ""
        print(f"    {task['id']}{priority} — {task['title']}{reason}")
    if len(tasks) > max_next:
        print(f"    ... +{len(tasks) - max_next} more")


def print_text(results: List[Dict[str, Any]], hidden_done: int, max_next: int) -> None:
    if not results:
        print("No Workflow Kit workspaces found for the selected filters.")
        return

    for status in GROUP_ORDER:
        group = [item for item in results if item["status"] == status]
        if not group:
            continue
        print(f"{status} ({len(group)})")
        for item in group:
            progress = item["progress"]
            if progress["total"]:
                progress_text = f"{progress['done']}/{progress['total']} done"
            else:
                progress_text = "no tasks"
            print(f"- {item['name']} — {progress_text}")

            if item["status"] == "READY":
                print_task_list("next", item["readyTasks"], max_next)
                if item.get("command"):
                    print(f"  command: {item['command']}")
            elif item["status"] == "BLOCKED":
                print_task_list("blocked", item["blockedTasks"], max_next)
            elif item.get("missingFiles"):
                print("  missing: " + ", ".join(item["missingFiles"]))
            if item.get("problems"):
                print("  problems: " + "; ".join(item["problems"]))
        print()

    if hidden_done:
        print(f"DONE hidden: {hidden_done} (--all to show)")


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    base_dir = specs_path(root, args.specs_dir)

    if not base_dir.is_dir():
        message = f"Specs directory not found: {base_dir}"
        if args.as_json:
            print(json.dumps({"error": message, "items": []}, ensure_ascii=False, indent=2))
        else:
            print(message, file=sys.stderr)
        return 1

    discovered = discover_workspaces(base_dir)
    results = [evaluate_workspace(workspace) for workspace in discovered]
    # Keep workspaces newest-first inside each status group. discover_workspaces()
    # already returns names in descending timestamp order; Python sort is stable.
    results.sort(key=lambda item: status_index(item["status"]))

    if args.next_task is not None:
        if args.next_task:
            workspace = resolve_workspace(root, base_dir, args.next_task).resolve()
            result = evaluate_workspace(workspace)
        else:
            result = next((item for item in results if item["status"] == "READY"), None)
            workspace = Path(result["path"]).resolve() if result else None

        task = None
        if result and workspace and result["readyTasks"]:
            task = load_first_ready_task(workspace, result["readyTasks"][0]["id"])
        print(json.dumps({"workspace": result, "task": task}, ensure_ascii=False, indent=2))
        return 0 if task else 1

    visible = filtered_results(results, args)
    hidden_done = 0 if args.all or args.status else sum(1 for item in results if item["status"] == "DONE")

    if args.as_json:
        payload = {
            "specsDir": str(base_dir),
            "items": visible,
            "summary": {
                status: sum(1 for item in results if item["status"] == status)
                for status in GROUP_ORDER
            },
            "hiddenDone": hidden_done,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print_text(visible, hidden_done, args.max_next)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
