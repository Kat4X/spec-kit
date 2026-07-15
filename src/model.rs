use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde_json::{Map, Value};

use crate::error::AppError;

pub const DONE_STATUSES: [&str; 2] = ["done", "skipped"];
const KNOWN_STATUSES: [&str; 5] = ["pending", "in_progress", "done", "blocked", "skipped"];
const CONFIRMATION_STATUSES: [&str; 4] = ["not_required", "pending", "approved", "rejected"];
const FILE_ACTIONS: [&str; 4] = ["read", "edit", "create", "delete"];

#[derive(Clone, Debug)]
pub struct TasksFile {
    pub raw: Value,
    pub version: u64,
    pub change: String,
    pub spec: String,
    pub plan: String,
    pub research: Option<String>,
    pub data_model: Option<String>,
    pub scope: String,
    pub verification: String,
    pub tasks: Vec<Task>,
    pub order: Vec<String>,
    pub first_task: String,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub refs: Vec<String>,
    pub depends_on: Vec<String>,
    pub parallel: bool,
    pub confirmation: Confirmation,
    pub files: BTreeMap<String, Vec<String>>,
    pub checks: Vec<Value>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct Confirmation {
    pub required: bool,
    pub status: String,
}

impl TasksFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        let raw: Value = serde_json::from_slice(bytes).map_err(|error| {
            AppError::Contract(format!(
                "tasks.json is invalid JSON at line {}, column {}: {}",
                error.line(),
                error.column(),
                error
            ))
        })?;
        Self::from_value(raw)
    }

    pub fn from_value(raw: Value) -> Result<Self, AppError> {
        let errors = validate_tasks_value(&raw);
        if !errors.is_empty() {
            return Err(AppError::Contract(format!(
                "tasks.json contract errors:\n- {}",
                errors.join("\n- ")
            )));
        }

        let object = raw.as_object().expect("validated object");
        let version = object["version"].as_u64().expect("validated version");
        let tasks = object["tasks"]
            .as_array()
            .expect("validated tasks")
            .iter()
            .map(|value| normalize_task(value, version))
            .collect();
        let execution = object["execution"]
            .as_object()
            .expect("validated execution");
        let change = string(object, "change");
        let spec = string(object, "spec");
        let plan = string(object, "plan");
        let research = optional_string(object, "research");
        let data_model = optional_string(object, "dataModel");
        let scope = string(object, "scope");
        let verification = string(object, "verification");
        let order = strings(execution, "order");
        let first_task = string(execution, "firstTask");

        Ok(Self {
            raw,
            version,
            change,
            spec,
            plan,
            research,
            data_model,
            scope,
            verification,
            tasks,
            order,
            first_task,
        })
    }

    pub fn ordered_tasks(&self) -> Vec<&Task> {
        self.order
            .iter()
            .filter_map(|task_id| self.task(task_id))
            .collect()
    }

    pub fn task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == task_id)
    }

    pub fn with_statuses(&self, task_ids: &[String], status: &str) -> Result<Self, AppError> {
        if !KNOWN_STATUSES.contains(&status) {
            return Err(AppError::Contract(format!("invalid task status: {status}")));
        }
        let mut raw = self.raw.clone();
        let tasks = raw
            .get_mut("tasks")
            .and_then(Value::as_array_mut)
            .expect("validated tasks");
        let requested: HashSet<&str> = task_ids.iter().map(String::as_str).collect();
        let mut updated = HashSet::new();
        for task in tasks {
            let Some(task_object) = task.as_object_mut() else {
                continue;
            };
            let Some(task_id) = task_object
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            if requested.contains(task_id.as_str()) {
                task_object.insert("status".into(), Value::String(status.into()));
                updated.insert(task_id);
            }
        }
        let missing: Vec<_> = task_ids
            .iter()
            .filter(|task_id| !updated.contains(task_id.as_str()))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(AppError::Contract(format!(
                "unknown task ids: {}",
                missing.join(", ")
            )));
        }
        Self::from_value(raw)
    }

    pub fn to_pretty_bytes(&self) -> Result<Vec<u8>, AppError> {
        let mut bytes = serde_json::to_vec_pretty(&self.raw)
            .map_err(|error| AppError::Internal(error.into()))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

impl Task {
    pub fn is_done(&self) -> bool {
        DONE_STATUSES.contains(&self.status.as_str())
    }
}

pub fn validate_tasks_value(value: &Value) -> Vec<String> {
    let Some(root) = value.as_object() else {
        return vec!["tasks.json must contain an object".into()];
    };
    let mut errors = Vec::new();
    let required_top = [
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
    ];
    missing_keys(root, &required_top, "tasks.json", &mut errors);

    let version = root.get("version").and_then(Value::as_u64);
    if !matches!(version, Some(1 | 2)) {
        errors.push(format!(
            "unsupported tasks.json version: {}",
            root.get("version").unwrap_or(&Value::Null)
        ));
        return errors;
    }
    let version = version.expect("checked version");

    for key in ["change", "spec", "plan", "scope", "verification"] {
        if !is_nonempty_string(root.get(key)) {
            errors.push(format!("{key} must be a non-empty string"));
        }
    }
    for key in ["research", "dataModel"] {
        if !matches!(root.get(key), Some(Value::Null | Value::String(_))) {
            errors.push(format!("{key} must be a string or null"));
        }
    }

    let Some(tasks) = root.get("tasks").and_then(Value::as_array) else {
        errors.push("tasks must be a non-empty array".into());
        return errors;
    };
    if tasks.is_empty() {
        errors.push("tasks must be a non-empty array".into());
        return errors;
    }
    for (index, task) in tasks.iter().enumerate() {
        validate_task(task, index, version, &mut errors);
    }

    let ids: Vec<_> = tasks
        .iter()
        .map(|task| {
            task.get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    let valid_ids: Vec<_> = ids
        .iter()
        .filter(|task_id| valid_task_id(task_id))
        .cloned()
        .collect();
    if valid_ids.iter().collect::<HashSet<_>>().len() != valid_ids.len() {
        errors.push("task ids must be unique".into());
    }
    let expected_ids: Vec<_> = (1..=tasks.len())
        .map(|index| format!("T{index:03}"))
        .collect();
    if ids != expected_ids {
        errors.push("task ids must be monotonic T001..Txxx without gaps in tasks array".into());
    }

    let mut order = Vec::new();
    match root.get("execution").and_then(Value::as_object) {
        None => errors.push("execution must be an object".into()),
        Some(execution) => {
            match string_array(execution.get("order"), false) {
                None => errors.push("execution.order must be a non-empty string array".into()),
                Some(raw_order) => {
                    order = raw_order;
                    let order_set: BTreeSet<_> = order.iter().collect();
                    let id_set: BTreeSet<_> = valid_ids.iter().collect();
                    if order.len() != order_set.len() || order_set != id_set {
                        errors
                            .push("execution.order must contain every task id exactly once".into());
                    }
                }
            }
            let first_task = execution.get("firstTask").and_then(Value::as_str);
            if order.first().map(String::as_str) != first_task {
                errors
                    .push("execution.firstTask must equal the first id in execution.order".into());
            }
        }
    }

    let id_set: HashSet<_> = valid_ids.iter().map(String::as_str).collect();
    let order_index: BTreeMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(index, task_id)| (task_id.as_str(), index))
        .collect();
    for task in tasks {
        let Some(task_id) = task.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(deps) = string_array(task.get("dependsOn"), true) else {
            continue;
        };
        let unknown: Vec<_> = deps
            .iter()
            .filter(|dep| !id_set.contains(dep.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            errors.push(format!(
                "{task_id} has unknown dependencies: {}",
                unknown.join(", ")
            ));
        }
        if deps.iter().any(|dep| dep == task_id) {
            errors.push(format!("{task_id} cannot depend on itself"));
        }
        if let Some(task_position) = order_index.get(task_id) {
            let later: Vec<_> = deps
                .iter()
                .filter(|dep| {
                    order_index
                        .get(dep.as_str())
                        .is_some_and(|position| position >= task_position)
                })
                .cloned()
                .collect();
            if !later.is_empty() {
                errors.push(format!(
                    "{task_id} dependencies must precede it in execution.order: {}",
                    later.join(", ")
                ));
            }
        }
    }

    if !matches!(root.get("coverage"), Some(Value::Array(_))) {
        errors.push("coverage must be an array".into());
    }
    validate_summary(root.get("summary"), tasks, version, &mut errors);
    errors
}

fn validate_task(value: &Value, index: usize, version: u64, errors: &mut Vec<String>) {
    let label = format!("tasks[{index}]");
    let Some(task) = value.as_object() else {
        errors.push(format!("{label} must be an object"));
        return;
    };
    let mut required = vec![
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
    ];
    if version == 1 {
        required.push("confirmationRequired");
    }
    missing_keys(task, &required, &label, errors);

    if !task
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(valid_task_id)
    {
        errors.push(format!("{label}.id must match Txxx"));
    }
    for key in ["title", "details", "priority"] {
        if !is_nonempty_string(task.get(key)) {
            errors.push(format!("{label}.{key} must be a non-empty string"));
        }
    }
    if !task
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| KNOWN_STATUSES.contains(&status))
    {
        errors.push(format!(
            "{label}.status is invalid: {}",
            task.get("status").unwrap_or(&Value::Null)
        ));
    }
    if string_array(task.get("refs"), false).is_none() {
        errors.push(format!("{label}.refs must be a non-empty string array"));
    }
    if string_array(task.get("dependsOn"), true).is_none() {
        errors.push(format!("{label}.dependsOn must be a string array"));
    }
    if !matches!(task.get("parallel"), Some(Value::Bool(_))) {
        errors.push(format!("{label}.parallel must be a boolean"));
    }

    match task.get("files").and_then(Value::as_object) {
        None => errors.push(format!(
            "{label}.files must contain exactly read/edit/create/delete"
        )),
        Some(files) => {
            let keys: BTreeSet<_> = files.keys().map(String::as_str).collect();
            let expected: BTreeSet<_> = FILE_ACTIONS.into_iter().collect();
            if keys != expected {
                errors.push(format!(
                    "{label}.files must contain exactly read/edit/create/delete"
                ));
            } else if FILE_ACTIONS
                .iter()
                .any(|action| string_array(files.get(*action), true).is_none())
            {
                errors.push(format!("{label}.files values must be string arrays"));
            }
        }
    }

    match task.get("checks").and_then(Value::as_array) {
        None => errors.push(format!("{label}.checks must be a non-empty array")),
        Some(checks) if checks.is_empty() => {
            errors.push(format!("{label}.checks must be a non-empty array"));
        }
        Some(checks) => {
            for (check_index, check) in checks.iter().enumerate() {
                let check_label = format!("{label}.checks[{check_index}]");
                let Some(check) = check.as_object() else {
                    errors.push(format!("{check_label} must be an object"));
                    continue;
                };
                if !check
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| matches!(kind, "automated" | "manual"))
                {
                    errors.push(format!("{check_label}.type must be automated or manual"));
                }
                if !matches!(check.get("required"), Some(Value::Bool(_))) {
                    errors.push(format!("{check_label}.required must be a boolean"));
                }
                if string_array(check.get("covers"), false).is_none() {
                    errors.push(format!(
                        "{check_label}.covers must be a non-empty string array"
                    ));
                }
                for field in ["command", "scenario"] {
                    if !matches!(check.get(field), Some(Value::Null | Value::String(_))) {
                        errors.push(format!("{check_label}.{field} must be a string or null"));
                    }
                }
                if !is_nonempty_string(check.get("command"))
                    && !is_nonempty_string(check.get("scenario"))
                {
                    errors.push(format!("{check_label} needs a command or scenario"));
                }
            }
        }
    }

    validate_verification(task.get("verification"), &label, errors);
    validate_confirmation(task, version, &label, errors);
}

fn validate_confirmation(
    task: &Map<String, Value>,
    version: u64,
    label: &str,
    errors: &mut Vec<String>,
) {
    if version == 1 {
        if !matches!(task.get("confirmationRequired"), Some(Value::Bool(_))) {
            errors.push(format!(
                "{label}.confirmationRequired must be a boolean in schema v1"
            ));
        }
        if !matches!(
            task.get("confirmation"),
            Some(Value::Null | Value::String(_))
        ) {
            errors.push(format!(
                "{label}.confirmation must be a string or null in schema v1"
            ));
        }
        if task.get("confirmationRequired").and_then(Value::as_bool) == Some(true)
            && !is_nonempty_string(task.get("confirmation"))
        {
            errors.push(format!(
                "{label}.confirmation must describe the required decision"
            ));
        }
        return;
    }

    let Some(confirmation) = task.get("confirmation").and_then(Value::as_object) else {
        errors.push(format!(
            "{label}.confirmation must be an object in schema v2"
        ));
        return;
    };
    let required_keys = ["required", "status", "request", "evidence", "updatedAt"];
    missing_keys(
        confirmation,
        &required_keys,
        &format!("{label}.confirmation"),
        errors,
    );
    let required = confirmation.get("required").and_then(Value::as_bool);
    let status = confirmation.get("status").and_then(Value::as_str);
    if required.is_none() {
        errors.push(format!("{label}.confirmation.required must be a boolean"));
    }
    if !status.is_some_and(|value| CONFIRMATION_STATUSES.contains(&value)) {
        errors.push(format!(
            "{label}.confirmation.status is invalid: {}",
            confirmation.get("status").unwrap_or(&Value::Null)
        ));
    }
    if !matches!(
        confirmation.get("request"),
        Some(Value::Null | Value::String(_))
    ) {
        errors.push(format!(
            "{label}.confirmation.request must be a string or null"
        ));
    }
    if !matches!(confirmation.get("evidence"), Some(Value::String(_))) {
        errors.push(format!("{label}.confirmation.evidence must be a string"));
    }
    if !matches!(
        confirmation.get("updatedAt"),
        Some(Value::Null | Value::String(_))
    ) {
        errors.push(format!(
            "{label}.confirmation.updatedAt must be a string or null"
        ));
    }
    if let (Some(required), Some(status)) = (required, status) {
        if !required && status != "not_required" {
            errors.push(format!(
                "{label}.confirmation.status must be not_required when required is false"
            ));
        }
        if required && status == "not_required" {
            errors.push(format!(
                "{label}.confirmation.status cannot be not_required when required is true"
            ));
        }
        if required && !is_nonempty_string(confirmation.get("request")) {
            errors.push(format!("{label}.confirmation.request is required"));
        }
        if matches!(status, "approved" | "rejected") {
            if !is_nonempty_string(confirmation.get("evidence")) {
                errors.push(format!(
                    "{label}.confirmation.evidence is required for {status}"
                ));
            }
            if !is_nonempty_string(confirmation.get("updatedAt")) {
                errors.push(format!(
                    "{label}.confirmation.updatedAt is required for {status}"
                ));
            }
        }
    }
}

fn validate_verification(value: Option<&Value>, label: &str, errors: &mut Vec<String>) {
    let Some(verification) = value.and_then(Value::as_object) else {
        errors.push(format!("{label}.verification must be an object"));
        return;
    };
    if !verification
        .get("result")
        .and_then(Value::as_str)
        .is_some_and(|result| matches!(result, "not_run" | "passed" | "failed" | "covered"))
    {
        errors.push(format!("{label}.verification.result is invalid"));
    }
    if !matches!(verification.get("evidence"), Some(Value::String(_))) {
        errors.push(format!("{label}.verification.evidence must be a string"));
    }
    if !matches!(
        verification.get("updatedAt"),
        Some(Value::Null | Value::String(_))
    ) {
        errors.push(format!(
            "{label}.verification.updatedAt must be a string or null"
        ));
    }
}

fn validate_summary(
    value: Option<&Value>,
    tasks: &[Value],
    version: u64,
    errors: &mut Vec<String>,
) {
    let Some(summary) = value.and_then(Value::as_object) else {
        errors.push("summary must be an object".into());
        return;
    };
    let parallel = tasks
        .iter()
        .filter(|task| task.get("parallel").and_then(Value::as_bool) == Some(true))
        .count() as u64;
    let confirmation_required = tasks
        .iter()
        .filter(|task| {
            if version == 1 {
                task.get("confirmationRequired").and_then(Value::as_bool) == Some(true)
            } else {
                task.pointer("/confirmation/required")
                    .and_then(Value::as_bool)
                    == Some(true)
            }
        })
        .count() as u64;
    if summary.get("total").and_then(Value::as_u64) != Some(tasks.len() as u64) {
        errors.push("summary.total does not match tasks count".into());
    }
    if summary.get("parallel").and_then(Value::as_u64) != Some(parallel) {
        errors.push("summary.parallel does not match tasks".into());
    }
    if summary.get("confirmationRequired").and_then(Value::as_u64) != Some(confirmation_required) {
        errors.push("summary.confirmationRequired does not match tasks".into());
    }
    if !matches!(summary.get("readyForImplement"), Some(Value::Bool(_))) {
        errors.push("summary.readyForImplement must be a boolean".into());
    }
}

fn normalize_task(value: &Value, version: u64) -> Task {
    let task = value.as_object().expect("validated task");
    let confirmation = if version == 1 {
        let required = task["confirmationRequired"]
            .as_bool()
            .expect("validated confirmationRequired");
        Confirmation {
            required,
            status: if required { "pending" } else { "not_required" }.into(),
        }
    } else {
        let confirmation = task["confirmation"]
            .as_object()
            .expect("validated confirmation");
        Confirmation {
            required: confirmation["required"]
                .as_bool()
                .expect("validated confirmation.required"),
            status: confirmation["status"]
                .as_str()
                .expect("validated confirmation.status")
                .into(),
        }
    };
    let files = task["files"]
        .as_object()
        .expect("validated files")
        .iter()
        .map(|(action, paths)| {
            (
                action.clone(),
                paths
                    .as_array()
                    .expect("validated file paths")
                    .iter()
                    .map(|path| path.as_str().expect("validated file path").to_owned())
                    .collect(),
            )
        })
        .collect();
    Task {
        id: string(task, "id"),
        title: string(task, "title"),
        status: string(task, "status"),
        priority: string(task, "priority"),
        refs: strings(task, "refs"),
        depends_on: strings(task, "dependsOn"),
        parallel: task["parallel"].as_bool().expect("validated parallel"),
        confirmation,
        files,
        checks: task["checks"].as_array().expect("validated checks").clone(),
        raw: value.clone(),
    }
}

fn missing_keys(
    object: &Map<String, Value>,
    required: &[&str],
    label: &str,
    errors: &mut Vec<String>,
) {
    let missing: Vec<_> = required
        .iter()
        .filter(|key| !object.contains_key(**key))
        .copied()
        .collect();
    if !missing.is_empty() {
        errors.push(format!("{label} is missing: {}", missing.join(", ")));
    }
}

fn valid_task_id(value: &str) -> bool {
    value.len() == 4
        && value.starts_with('T')
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
}

fn is_nonempty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn string_array(value: Option<&Value>, allow_empty: bool) -> Option<Vec<String>> {
    let values = value?.as_array()?;
    if !allow_empty && values.is_empty() {
        return None;
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
        })
        .collect()
}

fn string(object: &Map<String, Value>, key: &str) -> String {
    object[key].as_str().expect("validated string").to_owned()
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object[key].as_str().map(str::to_owned)
}

fn strings(object: &Map<String, Value>, key: &str) -> Vec<String> {
    object[key]
        .as_array()
        .expect("validated string array")
        .iter()
        .map(|value| value.as_str().expect("validated string").to_owned())
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::{Value, json};

    use super::TasksFile;

    pub fn task(id: &str) -> Value {
        json!({
            "id": id,
            "title": format!("Task {id}"),
            "details": "Implement behavior.",
            "status": "pending",
            "priority": "P0",
            "refs": ["R-001"],
            "dependsOn": [],
            "parallel": false,
            "confirmation": {
                "required": false,
                "status": "not_required",
                "request": null,
                "evidence": "",
                "updatedAt": null
            },
            "files": {"read": [], "edit": ["file.txt"], "create": [], "delete": []},
            "checks": [{
                "type": "automated",
                "command": "true",
                "scenario": null,
                "required": true,
                "covers": ["R-001"]
            }],
            "verification": {"result": "not_run", "evidence": "", "updatedAt": null}
        })
    }

    pub fn document(tasks: Vec<Value>) -> Value {
        let order: Vec<_> = tasks
            .iter()
            .map(|task| task["id"].as_str().unwrap().to_owned())
            .collect();
        let parallel = tasks.iter().filter(|task| task["parallel"] == true).count();
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
                "confirmationRequired": 0,
                "readyForImplement": true
            }
        })
    }

    #[test]
    fn rejects_boolean_schema_version() {
        let mut value = document(vec![task("T001")]);
        value["version"] = json!(true);
        let error = TasksFile::from_value(value).unwrap_err().to_string();
        assert!(error.contains("unsupported tasks.json version"));
    }

    #[test]
    fn rejects_dependency_after_task_in_execution_order() {
        let first = task("T001");
        let mut second = task("T002");
        second["dependsOn"] = json!(["T001"]);
        let mut value = document(vec![first, second]);
        value["execution"]["order"] = json!(["T002", "T001"]);
        value["execution"]["firstTask"] = json!("T002");
        let error = TasksFile::from_value(value).unwrap_err().to_string();
        assert!(error.contains("dependencies must precede"));
    }

    #[test]
    fn normalizes_schema_v1_confirmation() {
        let mut value = document(vec![task("T001")]);
        value["version"] = json!(1);
        value["tasks"][0]["confirmationRequired"] = json!(true);
        value["tasks"][0]["confirmation"] = json!("Approve config");
        value["summary"]["confirmationRequired"] = json!(1);
        let parsed = TasksFile::from_value(value).unwrap();
        assert!(parsed.tasks[0].confirmation.required);
        assert_eq!(parsed.tasks[0].confirmation.status, "pending");
    }

    #[test]
    fn status_update_preserves_unknown_fields() {
        let mut value = document(vec![task("T001")]);
        value["extension"] = json!({"kept": true});
        let parsed = TasksFile::from_value(value).unwrap();
        let updated = parsed
            .with_statuses(&["T001".into()], "in_progress")
            .unwrap();
        assert_eq!(updated.raw["extension"]["kept"], true);
        assert_eq!(updated.tasks[0].status, "in_progress");
    }
}
