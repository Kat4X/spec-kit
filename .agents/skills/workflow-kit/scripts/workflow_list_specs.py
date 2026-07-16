#!/usr/bin/env python3
"""List Workflow Kit workspaces ready for implementation.

The scanner is intentionally dependency-free: it only reads ai/specs/* and
computes readiness from tasks.json and required workflow artifacts.
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

DONE_STATUSES = {"done", "skipped"}
KNOWN_STATUSES = {"pending", "in_progress", "done", "blocked", "skipped"}
CONFIRMATION_STATUSES = {"not_required", "pending", "approved", "rejected"}
TASK_ID_RE = re.compile(r"^T[0-9]{3}$")
SUPPORTED_SCHEMA_VERSIONS = {1, 2}
TASK_KEYS = {
    "id",
    "title",
    "details",
    "status",
    "priority",
    "refs",
    "dependsOn",
    "parallel",
    "confirmation",
    "files",
    "checks",
    "verification",
}
FILE_ACTIONS = {"read", "edit", "create", "delete"}
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
    parser.add_argument(
        "--task-id",
        metavar="TASK_ID",
        help="With --next-task WORKSPACE, print this task instead of the first runnable task.",
    )
    parser.add_argument(
        "--batch-queue",
        metavar="WORKSPACE",
        help="Print a simulated dependency-ready batch queue as JSON.",
    )
    return parser.parse_args()


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


def is_string_list(value: Any, *, allow_empty: bool = True) -> bool:
    return (
        isinstance(value, list)
        and (allow_empty or bool(value))
        and all(isinstance(item, str) and item.strip() for item in value)
    )


def confirmation_state(task: Dict[str, Any], version: int) -> Tuple[bool, str]:
    if version == 1:
        required = task["confirmationRequired"]
        return required, "pending" if required else "not_required"
    confirmation = task["confirmation"]
    return confirmation["required"], confirmation["status"]


def validate_confirmation(task: Dict[str, Any], version: int, label: str) -> List[str]:
    errors: List[str] = []
    if version == 1:
        if not isinstance(task.get("confirmationRequired"), bool):
            errors.append(f"{label}.confirmationRequired must be a boolean in schema v1")
        value = task.get("confirmation")
        if value is not None and not isinstance(value, str):
            errors.append(f"{label}.confirmation must be a string or null in schema v1")
        if task.get("confirmationRequired") is True and not clean_string(value):
            errors.append(f"{label}.confirmation must describe the required decision")
        return errors

    confirmation = task.get("confirmation")
    if not isinstance(confirmation, dict):
        return [f"{label}.confirmation must be an object in schema v2"]

    required_keys = {"required", "status", "request", "evidence", "updatedAt"}
    missing = sorted(required_keys - set(confirmation))
    if missing:
        errors.append(f"{label}.confirmation is missing: {', '.join(missing)}")
        return errors

    required = confirmation.get("required")
    status = confirmation.get("status")
    request = confirmation.get("request")
    evidence = confirmation.get("evidence")
    updated_at = confirmation.get("updatedAt")
    if not isinstance(required, bool):
        errors.append(f"{label}.confirmation.required must be a boolean")
    if status not in CONFIRMATION_STATUSES:
        errors.append(f"{label}.confirmation.status is invalid: {status!r}")
    if request is not None and not isinstance(request, str):
        errors.append(f"{label}.confirmation.request must be a string or null")
    if not isinstance(evidence, str):
        errors.append(f"{label}.confirmation.evidence must be a string")
    if updated_at is not None and not isinstance(updated_at, str):
        errors.append(f"{label}.confirmation.updatedAt must be a string or null")

    if isinstance(required, bool) and status in CONFIRMATION_STATUSES:
        if not required and status != "not_required":
            errors.append(f"{label}.confirmation.status must be not_required when required is false")
        if required and status == "not_required":
            errors.append(f"{label}.confirmation.status cannot be not_required when required is true")
        if required and not clean_string(request):
            errors.append(f"{label}.confirmation.request is required")
        if status in {"approved", "rejected"}:
            if not clean_string(evidence):
                errors.append(f"{label}.confirmation.evidence is required for {status}")
            if not clean_string(updated_at):
                errors.append(f"{label}.confirmation.updatedAt is required for {status}")
    return errors


def validate_task(task: Any, index: int, version: int) -> List[str]:
    label = f"tasks[{index}]"
    if not isinstance(task, dict):
        return [f"{label} must be an object"]

    expected_keys = set(TASK_KEYS)
    if version == 1:
        expected_keys.add("confirmationRequired")
    missing = sorted(expected_keys - set(task))
    errors = [f"{label} is missing: {', '.join(missing)}"] if missing else []

    task_id = task.get("id")
    if not isinstance(task_id, str) or not TASK_ID_RE.fullmatch(task_id):
        errors.append(f"{label}.id must match Txxx")
    for key in ("title", "details", "priority"):
        if not isinstance(task.get(key), str) or not task[key].strip():
            errors.append(f"{label}.{key} must be a non-empty string")
    status = task.get("status")
    if status not in KNOWN_STATUSES:
        errors.append(f"{label}.status is invalid: {status!r}")
    if not is_string_list(task.get("refs"), allow_empty=False):
        errors.append(f"{label}.refs must be a non-empty string array")
    if not is_string_list(task.get("dependsOn")):
        errors.append(f"{label}.dependsOn must be a string array")
    if not isinstance(task.get("parallel"), bool):
        errors.append(f"{label}.parallel must be a boolean")

    files = task.get("files")
    if not isinstance(files, dict) or set(files) != FILE_ACTIONS:
        errors.append(f"{label}.files must contain exactly read/edit/create/delete")
    elif any(not is_string_list(files[action]) for action in FILE_ACTIONS):
        errors.append(f"{label}.files values must be string arrays")

    checks = task.get("checks")
    if not isinstance(checks, list) or not checks:
        errors.append(f"{label}.checks must be a non-empty array")
    else:
        for check_index, check in enumerate(checks):
            check_label = f"{label}.checks[{check_index}]"
            if not isinstance(check, dict):
                errors.append(f"{check_label} must be an object")
                continue
            if check.get("type") not in {"automated", "manual"}:
                errors.append(f"{check_label}.type must be automated or manual")
            if not isinstance(check.get("required"), bool):
                errors.append(f"{check_label}.required must be a boolean")
            if not is_string_list(check.get("covers"), allow_empty=False):
                errors.append(f"{check_label}.covers must be a non-empty string array")
            command = check.get("command")
            scenario = check.get("scenario")
            if command is not None and not isinstance(command, str):
                errors.append(f"{check_label}.command must be a string or null")
            if scenario is not None and not isinstance(scenario, str):
                errors.append(f"{check_label}.scenario must be a string or null")
            if not clean_string(command) and not clean_string(scenario):
                errors.append(f"{check_label} needs a command or scenario")

    verification = task.get("verification")
    if not isinstance(verification, dict):
        errors.append(f"{label}.verification must be an object")
    else:
        if verification.get("result") not in {"not_run", "passed", "failed", "covered"}:
            errors.append(f"{label}.verification.result is invalid")
        if not isinstance(verification.get("evidence"), str):
            errors.append(f"{label}.verification.evidence must be a string")
        updated_at = verification.get("updatedAt")
        if updated_at is not None and not isinstance(updated_at, str):
            errors.append(f"{label}.verification.updatedAt must be a string or null")

    errors.extend(validate_confirmation(task, version, label))
    return errors


def validate_tasks_data(tasks_data: Any) -> List[str]:
    if not isinstance(tasks_data, dict):
        return ["tasks.json must contain an object"]

    required_top = {
        "version",
        "change",
        "spec",
        "plan",
        "research",
        "dataModel",
        "scope",
        "verification",
        "tasks",
        "execution",
        "coverage",
        "summary",
    }
    missing_top = sorted(required_top - set(tasks_data))
    errors = ["tasks.json is missing: " + ", ".join(missing_top)] if missing_top else []

    version = tasks_data.get("version")
    if type(version) is not int or version not in SUPPORTED_SCHEMA_VERSIONS:
        errors.append(f"unsupported tasks.json version: {version!r}")
        return errors
    for key in ("change", "spec", "plan", "scope", "verification"):
        if not isinstance(tasks_data.get(key), str) or not tasks_data[key].strip():
            errors.append(f"{key} must be a non-empty string")
    for key in ("research", "dataModel"):
        if tasks_data.get(key) is not None and not isinstance(tasks_data[key], str):
            errors.append(f"{key} must be a string or null")

    tasks = tasks_data.get("tasks")
    if not isinstance(tasks, list) or not tasks:
        errors.append("tasks must be a non-empty array")
        return errors
    for index, task in enumerate(tasks):
        errors.extend(validate_task(task, index, version))

    ids = [task.get("id") for task in tasks if isinstance(task, dict)]
    valid_ids = [task_id for task_id in ids if isinstance(task_id, str) and TASK_ID_RE.fullmatch(task_id)]
    if len(set(valid_ids)) != len(valid_ids):
        errors.append("task ids must be unique")
    expected_ids = [f"T{index:03d}" for index in range(1, len(tasks) + 1)]
    if ids != expected_ids:
        errors.append("task ids must be monotonic T001..Txxx without gaps in tasks array")

    execution = tasks_data.get("execution")
    order: List[str] = []
    if not isinstance(execution, dict):
        errors.append("execution must be an object")
    else:
        raw_order = execution.get("order")
        if not is_string_list(raw_order, allow_empty=False):
            errors.append("execution.order must be a non-empty string array")
        else:
            order = raw_order
            if len(order) != len(set(order)) or set(order) != set(valid_ids):
                errors.append("execution.order must contain every task id exactly once")
        first_task = execution.get("firstTask")
        if not isinstance(first_task, str) or not order or first_task != order[0]:
            errors.append("execution.firstTask must equal the first id in execution.order")

    id_set = set(valid_ids)
    order_index = {task_id: index for index, task_id in enumerate(order)}
    for task in tasks:
        if not isinstance(task, dict) or task.get("id") not in id_set:
            continue
        deps = task.get("dependsOn")
        if not isinstance(deps, list):
            continue
        unknown = [dep for dep in deps if dep not in id_set]
        if unknown:
            errors.append(f"{task['id']} has unknown dependencies: {', '.join(unknown)}")
        if task["id"] in deps:
            errors.append(f"{task['id']} cannot depend on itself")
        if task["id"] in order_index:
            later = [dep for dep in deps if dep in order_index and order_index[dep] >= order_index[task["id"]]]
            if later:
                errors.append(f"{task['id']} dependencies must precede it in execution.order: {', '.join(later)}")

    if not isinstance(tasks_data.get("coverage"), list):
        errors.append("coverage must be an array")
    summary = tasks_data.get("summary")
    if not isinstance(summary, dict):
        errors.append("summary must be an object")
    else:
        expected_parallel = sum(1 for task in tasks if isinstance(task, dict) and task.get("parallel") is True)
        expected_confirmation = 0
        for task in tasks:
            if not isinstance(task, dict):
                continue
            if version == 1 and task.get("confirmationRequired") is True:
                expected_confirmation += 1
            elif version == 2 and isinstance(task.get("confirmation"), dict) and task["confirmation"].get("required") is True:
                expected_confirmation += 1
        if type(summary.get("total")) is not int or summary["total"] != len(tasks):
            errors.append("summary.total does not match tasks count")
        if type(summary.get("parallel")) is not int or summary["parallel"] != expected_parallel:
            errors.append("summary.parallel does not match tasks")
        if (
            type(summary.get("confirmationRequired")) is not int
            or summary["confirmationRequired"] != expected_confirmation
        ):
            errors.append("summary.confirmationRequired does not match tasks")
        if not isinstance(summary.get("readyForImplement"), bool):
            errors.append("summary.readyForImplement must be a boolean")
    return errors


def ordered_tasks(tasks_data: Dict[str, Any]) -> List[Dict[str, Any]]:
    tasks_by_id = {task["id"]: task for task in tasks_data["tasks"]}
    return [tasks_by_id[task_id] for task_id in tasks_data["execution"]["order"]]


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

    schema_errors = validate_tasks_data(tasks_data)
    if schema_errors:
        result["status"] = "BROKEN"
        result["problems"].extend(schema_errors)
        return result

    version = tasks_data["version"]
    tasks = ordered_tasks(tasks_data)
    done_ids = {
        task["id"]
        for task in tasks
        if task["status"] in DONE_STATUSES
    }

    ready_tasks: List[Dict[str, Any]] = []
    blocked_tasks: List[Dict[str, Any]] = []
    status_counts = {"done": 0, "pending": 0, "blocked": 0, "in_progress": 0, "skipped": 0}
    unfinished_count = 0

    for raw_task in tasks:
        status = raw_task["status"]
        status_counts[status] += 1
        if status not in DONE_STATUSES:
            unfinished_count += 1

        if status == "pending":
            deps = raw_task["dependsOn"]
            unmet_deps = [dep for dep in deps if dep not in done_ids]
            reasons: List[str] = []
            if unmet_deps:
                reasons.append("unmet deps: " + ", ".join(unmet_deps))
            required, confirmation_status = confirmation_state(raw_task, version)
            if required and confirmation_status != "approved":
                reasons.append(f"confirmation {confirmation_status}")

            if reasons:
                blocked_tasks.append(task_summary(raw_task, "; ".join(reasons)))
            else:
                ready_tasks.append(task_summary(raw_task))
        elif status in {"blocked", "in_progress"}:
            blocked_tasks.append(task_summary(raw_task, f"status is {status}"))
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
        result["command"] = f"workflow-kit implement {workspace.name} {first_task['id']}"
    elif unfinished_count > 0:
        result["status"] = "BLOCKED"
    else:
        result["status"] = "DONE"

    return result


def load_tasks_data(workspace: Path) -> Optional[Dict[str, Any]]:
    try:
        with (workspace / "tasks.json").open("r", encoding="utf-8") as file:
            data = json.load(file)
    except (OSError, json.JSONDecodeError):
        return None
    if validate_tasks_data(data):
        return None
    return data


def build_batch_queue(tasks_data: Dict[str, Any]) -> Dict[str, List[Dict[str, Any]]]:
    version = tasks_data["version"]
    tasks = ordered_tasks(tasks_data)
    simulated_done = {task["id"] for task in tasks if task["status"] in DONE_STATUSES}
    queued_ids = set()
    queue: List[Dict[str, Any]] = []

    progressed = True
    while progressed:
        progressed = False
        for task in tasks:
            task_id = task["id"]
            if task["status"] != "pending" or task_id in queued_ids:
                continue
            required, status = confirmation_state(task, version)
            if required and status != "approved":
                continue
            if all(dep in simulated_done for dep in task["dependsOn"]):
                queue.append(task_summary(task))
                queued_ids.add(task_id)
                simulated_done.add(task_id)
                progressed = True

    blockers: List[Dict[str, Any]] = []
    for task in tasks:
        if task["status"] in DONE_STATUSES or task["id"] in queued_ids:
            continue
        reasons: List[str] = []
        if task["status"] != "pending":
            reasons.append(f"status is {task['status']}")
        else:
            required, status = confirmation_state(task, version)
            if required and status != "approved":
                reasons.append(f"confirmation {status}")
            unmet = [dep for dep in task["dependsOn"] if dep not in simulated_done]
            if unmet:
                reasons.append("unmet deps: " + ", ".join(unmet))
        blockers.append(task_summary(task, "; ".join(reasons) or "not runnable"))
    return {"queue": queue, "blockers": blockers}


def specs_path(root: Path, specs_dir: str) -> Path:
    candidate = Path(specs_dir)
    if candidate.is_absolute():
        return candidate
    return root / candidate


def resolve_workspace(root: Path, base_dir: Path, value: str) -> Path:
    candidate = Path(value)
    if candidate.is_absolute():
        return candidate
    named_workspace = base_dir / value
    if named_workspace.is_dir():
        return named_workspace
    repository_path = root / candidate
    if repository_path.is_dir():
        return repository_path
    return named_workspace


def load_task(workspace: Path, task_id: str) -> Optional[Dict[str, Any]]:
    tasks_data = load_tasks_data(workspace)
    if tasks_data is None:
        return None
    for task in tasks_data["tasks"]:
        if task["id"] == task_id:
            return task
    return None


def compact_workspace(result: Optional[Dict[str, Any]]) -> Optional[Dict[str, Any]]:
    if result is None:
        return None
    return {
        "name": result["name"],
        "path": result["path"],
        "status": result["status"],
        "progress": result["progress"],
        "missingFiles": result["missingFiles"],
        "problems": result["problems"],
        "command": result["command"],
    }


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
    if args.max_next < 0:
        print("--max-next must be zero or greater", file=sys.stderr)
        return 2
    if args.task_id and args.next_task is None:
        print("--task-id requires --next-task", file=sys.stderr)
        return 2
    if args.batch_queue and args.next_task is not None:
        print("--batch-queue cannot be combined with --next-task", file=sys.stderr)
        return 2
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

    if args.batch_queue:
        workspace = resolve_workspace(root, base_dir, args.batch_queue).resolve()
        result = evaluate_workspace(workspace)
        tasks_data = load_tasks_data(workspace) if result["status"] != "BROKEN" else None
        batch = build_batch_queue(tasks_data) if tasks_data else {"queue": [], "blockers": []}
        payload = {"workspace": compact_workspace(result), **batch}
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0 if tasks_data and batch["queue"] else 1

    if args.next_task is not None:
        if args.next_task:
            workspace = resolve_workspace(root, base_dir, args.next_task).resolve()
            result = evaluate_workspace(workspace)
        else:
            result = next((item for item in results if item["status"] == "READY"), None)
            workspace = Path(result["path"]).resolve() if result else None

        task = None
        if result and workspace and result["status"] != "BROKEN":
            task_id = args.task_id or (result["readyTasks"][0]["id"] if result["readyTasks"] else "")
            if task_id:
                task = load_task(workspace, task_id)
        print(json.dumps({"workspace": compact_workspace(result), "task": task}, ensure_ascii=False, indent=2))
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
