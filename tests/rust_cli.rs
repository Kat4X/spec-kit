use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};
use tempfile::TempDir;

struct Fixture {
    _directory: TempDir,
    root: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = TempDir::new().unwrap();
        let root = directory.path().to_path_buf();
        fs::create_dir_all(root.join("ai/specs")).unwrap();
        Self {
            _directory: directory,
            root,
        }
    }

    fn workspace(&self, name: &str, tasks: Vec<Value>) -> std::path::PathBuf {
        let workspace = self.root.join("ai/specs").join(name);
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            workspace.join("spec.md"),
            "# Spec\n\n### Case-1: Selected\n\nSELECTED_MARKER\n\n### Case-2: Unrelated\n\nUNRELATED_MARKER\n",
        )
        .unwrap();
        fs::write(
            workspace.join("plan.md"),
            "# Plan\n\n## Краткое описание\n\nBaseline.\n\n## Архитектура / поток\n\nFlow.\n\n## Unrelated\n\nUNRELATED_PLAN_MARKER\n",
        )
        .unwrap();
        fs::write(workspace.join("scope.md"), "# Scope\n\nAllowed.\n").unwrap();
        fs::write(workspace.join("verification.md"), "# Verification\n").unwrap();
        fs::write(
            workspace.join("tasks.json"),
            serde_json::to_vec_pretty(&tasks_document(tasks)).unwrap(),
        )
        .unwrap();
        workspace
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_workflow-kit"))
            .args(args)
            .arg("--root")
            .arg(&self.root)
            .output()
            .unwrap()
    }
}

fn task(id: &str, reference: &str) -> Value {
    json!({
        "id": id,
        "title": format!("Task {id}"),
        "details": format!("Implement {reference}."),
        "status": "pending",
        "priority": "P0",
        "refs": [reference],
        "dependsOn": [],
        "parallel": false,
        "confirmation": {
            "required": false,
            "status": "not_required",
            "request": null,
            "evidence": "",
            "updatedAt": null
        },
        "files": {"read": [], "edit": [format!("{id}.txt")], "create": [], "delete": []},
        "checks": [{
            "type": "automated",
            "command": "true",
            "scenario": null,
            "required": true,
            "covers": [reference]
        }],
        "verification": {"result": "not_run", "evidence": "", "updatedAt": null}
    })
}

fn tasks_document(tasks: Vec<Value>) -> Value {
    let order: Vec<_> = tasks
        .iter()
        .map(|task| task["id"].as_str().unwrap().to_owned())
        .collect();
    let parallel = tasks.iter().filter(|task| task["parallel"] == true).count();
    let confirmation_required = tasks
        .iter()
        .filter(|task| task["confirmation"]["required"] == true)
        .count();
    json!({
        "version": 2,
        "change": "fixture",
        "spec": "spec.md",
        "plan": "plan.md",
        "research": null,
        "dataModel": null,
        "scope": "scope.md",
        "verification": "verification.md",
        "tasks": tasks,
        "execution": {"order": order, "firstTask": order[0]},
        "coverage": [],
        "summary": {
            "total": order.len(),
            "required": order.len(),
            "parallel": parallel,
            "confirmationRequired": confirmation_required,
            "readyForImplement": true
        }
    })
}

fn stdout_json(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "machine success polluted stderr");
    serde_json::from_slice(&output.stdout).unwrap()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn create_and_scaffold_are_unicode_safe_unique_and_no_overwrite() {
    let fixture = Fixture::new();
    let first = stdout_json(&fixture.run(&["create", "Экспорт задач", "--json"]));
    let second = stdout_json(&fixture.run(&["create", "Экспорт задач", "--json"]));
    let first_path = std::path::PathBuf::from(first["path"].as_str().unwrap());
    assert_ne!(first["path"], second["path"]);
    assert!(
        first["workspace"]
            .as_str()
            .unwrap()
            .contains("экспорт-задач")
    );
    assert!(
        fs::read_to_string(first_path.join("spec.md"))
            .unwrap()
            .starts_with("# {Change Name}")
    );

    let workspace = path_string(&first_path);
    let scaffold =
        stdout_json(&fixture.run(&["scaffold", &workspace, "--phase", "plan", "--json"]));
    assert_eq!(scaffold["created"], json!(["plan.md", "scope.md"]));
    fs::write(first_path.join("plan.md"), "CUSTOM PLAN\n").unwrap();
    let repeated =
        stdout_json(&fixture.run(&["scaffold", &workspace, "--phase", "plan", "--json"]));
    assert_eq!(repeated["created"], json!([]));
    assert_eq!(
        fs::read_to_string(first_path.join("plan.md")).unwrap(),
        "CUSTOM PLAN\n"
    );
}

#[test]
fn packet_selects_single_or_parallel_context_and_reports_blockers() {
    let fixture = Fixture::new();
    let sequential = fixture.workspace(
        "2026.07.15_00:00_sequential",
        vec![task("T001", "Case-1"), task("T002", "Case-2")],
    );
    let output = fixture.run(&["packet", &path_string(&sequential), "--json"]);
    let packet = stdout_json(&output);
    assert_eq!(packet["packetVersion"], 1);
    assert_eq!(packet["mode"], "single");
    assert_eq!(packet["tasks"].as_array().unwrap().len(), 1);
    let serialized = serde_json::to_string(&packet).unwrap();
    assert!(serialized.contains("SELECTED_MARKER"));
    assert!(!serialized.contains("UNRELATED_MARKER"));
    assert!(!serialized.contains("UNRELATED_PLAN_MARKER"));

    let mut first = task("T001", "Case-1");
    first["parallel"] = json!(true);
    let mut second = task("T002", "Case-2");
    second["parallel"] = json!(true);
    let parallel = fixture.workspace("2026.07.15_00:01_parallel", vec![first, second]);
    let packet = stdout_json(&fixture.run(&["packet", &path_string(&parallel), "--json"]));
    assert_eq!(packet["mode"], "parallel");
    assert_eq!(packet["tasks"].as_array().unwrap().len(), 2);

    let mut blocked_task = task("T001", "Case-1");
    blocked_task["confirmation"] = json!({
        "required": true,
        "status": "pending",
        "request": "Approve config",
        "evidence": "",
        "updatedAt": null
    });
    let blocked = fixture.workspace("2026.07.15_00:02_blocked", vec![blocked_task]);
    let output = fixture.run(&["packet", &path_string(&blocked), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "no_work");
}

#[test]
fn packet_lists_missing_requirement_refs() {
    let fixture = Fixture::new();
    let workspace = fixture.workspace("2026.07.15_00:03_missing-ref", vec![task("T001", "R-404")]);
    let packet = stdout_json(&fixture.run(&["packet", &path_string(&workspace), "--json"]));
    assert_eq!(packet["context"]["missingRefs"], json!(["R-404"]));
    assert_eq!(packet["context"]["requirements"], json!([]));
}

#[test]
fn claim_supports_dry_run_atomic_update_and_stale_conflict_exit() {
    let fixture = Fixture::new();
    let workspace = fixture.workspace("2026.07.15_00:04_claim", vec![task("T001", "Case-1")]);
    let workspace_arg = path_string(&workspace);
    let packet = stdout_json(&fixture.run(&["packet", &workspace_arg, "--json"]));
    let revision = packet["revision"].as_str().unwrap();

    let dry_run = stdout_json(&fixture.run(&[
        "claim",
        &workspace_arg,
        "--revision",
        revision,
        "--dry-run",
        "--json",
    ]));
    assert_eq!(dry_run["dryRun"], true);
    let unchanged: Value =
        serde_json::from_slice(&fs::read(workspace.join("tasks.json")).unwrap()).unwrap();
    assert_eq!(unchanged["tasks"][0]["status"], "pending");

    let claimed =
        stdout_json(&fixture.run(&["claim", &workspace_arg, "--revision", revision, "--json"]));
    assert_eq!(claimed["packet"]["tasks"][0]["status"], "in_progress");
    let updated: Value =
        serde_json::from_slice(&fs::read(workspace.join("tasks.json")).unwrap()).unwrap();
    assert_eq!(updated["tasks"][0]["status"], "in_progress");

    let stale = fixture.run(&["claim", &workspace_arg, "--revision", revision, "--json"]);
    assert_eq!(stale.status.code(), Some(4));
    assert!(stale.stdout.is_empty());
    let error: Value = serde_json::from_slice(&stale.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "conflict");
}

#[test]
fn validate_preserves_lifecycle_statuses_and_flags_contract_errors() {
    let fixture = Fixture::new();

    let needs_plan = fixture.root.join("ai/specs/2026.07.15_00:03_needs-plan");
    fs::create_dir_all(&needs_plan).unwrap();
    fs::write(needs_plan.join("spec.md"), "# Spec\n").unwrap();
    let report = stdout_json(&fixture.run(&["validate", &path_string(&needs_plan), "--json"]));
    assert_eq!(report["status"], "NEEDS_PLAN");

    let needs_tasks = fixture.root.join("ai/specs/2026.07.15_00:04_needs-tasks");
    fs::create_dir_all(&needs_tasks).unwrap();
    for file in ["spec.md", "plan.md", "scope.md"] {
        fs::write(needs_tasks.join(file), format!("# {file}\n")).unwrap();
    }
    let report = stdout_json(&fixture.run(&["validate", &path_string(&needs_tasks), "--json"]));
    assert_eq!(report["status"], "NEEDS_TASKS");

    let list = stdout_json(&fixture.run(&["list", "--json"]));
    for (name, status) in [
        ("2026.07.15_00:03_needs-plan", "NEEDS_PLAN"),
        ("2026.07.15_00:04_needs-tasks", "NEEDS_TASKS"),
    ] {
        assert!(
            list["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| { item["name"] == name && item["status"] == status })
        );
    }

    let broken = fixture.workspace(
        "2026.07.15_00:05_broken-reference",
        vec![task("T001", "Case-1")],
    );
    fs::remove_file(broken.join("verification.md")).unwrap();
    let output = fixture.run(&["validate", &path_string(&broken), "--json"]);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "BROKEN");
}

#[test]
fn legacy_commands_validate_and_path_escape_keep_stable_contracts() {
    let fixture = Fixture::new();
    let first = task("T001", "Case-1");
    let mut second = task("T002", "Case-2");
    second["dependsOn"] = json!(["T001"]);
    let workspace = fixture.workspace("2026.07.15_00:05_legacy", vec![first, second]);
    let workspace_arg = path_string(&workspace);

    let list = stdout_json(&fixture.run(&["list", "--json"]));
    assert!(
        list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"] == "2026.07.15_00:05_legacy")
    );
    let next = stdout_json(&fixture.run(&["next-task", &workspace_arg, "--json"]));
    assert_eq!(next["task"]["id"], "T001");
    let batch = stdout_json(&fixture.run(&["batch-queue", &workspace_arg, "--json"]));
    assert_eq!(batch["queue"].as_array().unwrap().len(), 2);
    let validate = stdout_json(&fixture.run(&["validate", &workspace_arg, "--json"]));
    assert_eq!(validate["status"], "READY");

    let escape = fixture.run(&["packet", "../outside", "--json"]);
    assert_eq!(escape.status.code(), Some(3));
    assert!(escape.stdout.is_empty());
    let error: Value = serde_json::from_slice(&escape.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "contract");

    fs::write(workspace.join("tasks.json"), "{broken").unwrap();
    let broken = fixture.run(&["validate", &workspace_arg, "--json"]);
    assert_eq!(broken.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&broken.stdout).unwrap();
    assert_eq!(report["status"], "BROKEN");
}
