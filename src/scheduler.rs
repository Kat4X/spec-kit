use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{error::AppError, model::TasksFile};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: String,
    pub priority: String,
    pub title: String,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub pending: usize,
    pub blocked: usize,
    pub in_progress: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug)]
pub struct QueueEvaluation {
    pub progress: Progress,
    pub ready_tasks: Vec<TaskSummary>,
    pub blocked_tasks: Vec<TaskSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitMode {
    Single,
    Parallel,
}

#[derive(Clone, Debug)]
pub struct ReadyUnit {
    pub mode: UnitMode,
    pub task_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchQueue {
    pub queue: Vec<TaskSummary>,
    pub blockers: Vec<TaskSummary>,
}

pub fn evaluate(document: &TasksFile) -> QueueEvaluation {
    let done_ids: HashSet<_> = document
        .tasks
        .iter()
        .filter(|task| task.is_done())
        .map(|task| task.id.as_str())
        .collect();
    let mut progress = Progress {
        total: document.tasks.len(),
        ..Progress::default()
    };
    let mut ready_tasks = Vec::new();
    let mut blocked_tasks = Vec::new();

    for task in document.ordered_tasks() {
        match task.status.as_str() {
            "done" => progress.done += 1,
            "skipped" => {
                progress.done += 1;
                progress.skipped += 1;
            }
            "pending" => progress.pending += 1,
            "blocked" => progress.blocked += 1,
            "in_progress" => progress.in_progress += 1,
            _ => {}
        }
        if task.status == "pending" {
            let reasons = readiness_reasons(task, &done_ids);
            if reasons.is_empty() {
                ready_tasks.push(summary(task, ""));
            } else {
                blocked_tasks.push(summary(task, &reasons.join("; ")));
            }
        } else if matches!(task.status.as_str(), "blocked" | "in_progress") {
            blocked_tasks.push(summary(task, &format!("status is {}", task.status)));
        }
    }
    QueueEvaluation {
        progress,
        ready_tasks,
        blocked_tasks,
    }
}

pub fn ready_unit(
    document: &TasksFile,
    explicit_task_id: Option<&str>,
) -> Result<ReadyUnit, AppError> {
    let done_ids: HashSet<_> = document
        .tasks
        .iter()
        .filter(|task| task.is_done())
        .map(|task| task.id.as_str())
        .collect();

    if let Some(task_id) = explicit_task_id {
        let task = document
            .task(task_id)
            .ok_or_else(|| AppError::Contract(format!("unknown task id: {task_id}")))?;
        let reasons = readiness_reasons(task, &done_ids);
        if task.status != "pending" || !reasons.is_empty() {
            let reason = if task.status != "pending" {
                format!("status is {}", task.status)
            } else {
                reasons.join("; ")
            };
            return Err(AppError::NoWork(format!(
                "task {task_id} is not runnable: {reason}"
            )));
        }
        return Ok(ReadyUnit {
            mode: UnitMode::Single,
            task_ids: vec![task_id.to_owned()],
        });
    }

    let ordered = document.ordered_tasks();
    let Some(first_index) = ordered
        .iter()
        .position(|task| task.status == "pending" && readiness_reasons(task, &done_ids).is_empty())
    else {
        return Err(AppError::NoWork(
            "workspace has no dependency-ready pending tasks".into(),
        ));
    };
    let first = ordered[first_index];
    if !first.parallel {
        return Ok(ReadyUnit {
            mode: UnitMode::Single,
            task_ids: vec![first.id.clone()],
        });
    }

    let mut task_ids = Vec::new();
    for task in ordered.into_iter().skip(first_index) {
        if task.is_done() {
            continue;
        }
        let is_ready = task.status == "pending" && readiness_reasons(task, &done_ids).is_empty();
        if is_ready && task.parallel {
            task_ids.push(task.id.clone());
        } else {
            break;
        }
    }
    Ok(ReadyUnit {
        mode: UnitMode::Parallel,
        task_ids,
    })
}

pub fn build_batch_queue(document: &TasksFile) -> BatchQueue {
    let ordered = document.ordered_tasks();
    let mut simulated_done: HashSet<_> = ordered
        .iter()
        .filter(|task| task.is_done())
        .map(|task| task.id.clone())
        .collect();
    let mut queued = HashSet::new();
    let mut queue = Vec::new();

    loop {
        let mut progressed = false;
        for task in &ordered {
            if task.status != "pending" || queued.contains(&task.id) {
                continue;
            }
            if task.confirmation.required && task.confirmation.status != "approved" {
                continue;
            }
            if task
                .depends_on
                .iter()
                .all(|dependency| simulated_done.contains(dependency))
            {
                queue.push(summary(task, ""));
                queued.insert(task.id.clone());
                simulated_done.insert(task.id.clone());
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }

    let mut blockers = Vec::new();
    for task in ordered {
        if task.is_done() || queued.contains(&task.id) {
            continue;
        }
        let mut reasons = Vec::new();
        if task.status != "pending" {
            reasons.push(format!("status is {}", task.status));
        } else {
            if task.confirmation.required && task.confirmation.status != "approved" {
                reasons.push(format!("confirmation {}", task.confirmation.status));
            }
            let unmet: Vec<_> = task
                .depends_on
                .iter()
                .filter(|dependency| !simulated_done.contains(*dependency))
                .cloned()
                .collect();
            if !unmet.is_empty() {
                reasons.push(format!("unmet deps: {}", unmet.join(", ")));
            }
        }
        blockers.push(summary(
            task,
            &if reasons.is_empty() {
                "not runnable".into()
            } else {
                reasons.join("; ")
            },
        ));
    }
    BatchQueue { queue, blockers }
}

pub fn selected_tasks<'a>(
    document: &'a TasksFile,
    unit: &ReadyUnit,
) -> Result<Vec<&'a crate::model::Task>, AppError> {
    let by_id: HashMap<_, _> = document
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    unit.task_ids
        .iter()
        .map(|task_id| {
            by_id
                .get(task_id.as_str())
                .copied()
                .ok_or_else(|| AppError::Contract(format!("unknown selected task: {task_id}")))
        })
        .collect()
}

fn readiness_reasons(task: &crate::model::Task, done_ids: &HashSet<&str>) -> Vec<String> {
    let mut reasons = Vec::new();
    if task.status != "pending" {
        reasons.push(format!("status is {}", task.status));
        return reasons;
    }
    let unmet: Vec<_> = task
        .depends_on
        .iter()
        .filter(|dependency| !done_ids.contains(dependency.as_str()))
        .cloned()
        .collect();
    if !unmet.is_empty() {
        reasons.push(format!("unmet deps: {}", unmet.join(", ")));
    }
    if task.confirmation.required && task.confirmation.status != "approved" {
        reasons.push(format!("confirmation {}", task.confirmation.status));
    }
    reasons
}

fn summary(task: &crate::model::Task, reason: &str) -> TaskSummary {
    TaskSummary {
        id: task.id.clone(),
        priority: task.priority.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::model::{TasksFile, tests::task};

    use super::{UnitMode, build_batch_queue, evaluate, ready_unit};

    fn parsed(tasks: Vec<serde_json::Value>) -> TasksFile {
        TasksFile::from_value(crate::model::tests::document(tasks)).unwrap()
    }

    #[test]
    fn execution_order_controls_ready_task() {
        let mut value = crate::model::tests::document(vec![task("T001"), task("T002")]);
        value["execution"]["order"] = json!(["T002", "T001"]);
        value["execution"]["firstTask"] = json!("T002");
        let document = TasksFile::from_value(value).unwrap();
        let evaluation = evaluate(&document);
        assert_eq!(evaluation.ready_tasks[0].id, "T002");
    }

    #[test]
    fn returns_parallel_ready_prefix_until_sequential_barrier() {
        let mut first = task("T001");
        first["parallel"] = json!(true);
        let mut second = task("T002");
        second["parallel"] = json!(true);
        let third = task("T003");
        let document = parsed(vec![first, second, third]);
        let unit = ready_unit(&document, None).unwrap();
        assert_eq!(unit.mode, UnitMode::Parallel);
        assert_eq!(unit.task_ids, ["T001", "T002"]);
    }

    #[test]
    fn explicit_parallel_task_is_a_single_unit() {
        let mut first = task("T001");
        first["parallel"] = json!(true);
        let document = parsed(vec![first]);
        let unit = ready_unit(&document, Some("T001")).unwrap();
        assert_eq!(unit.mode, UnitMode::Single);
        assert_eq!(unit.task_ids, ["T001"]);
    }

    #[test]
    fn confirmation_blocks_readiness() {
        let mut first = task("T001");
        first["confirmation"] = json!({
            "required": true,
            "status": "pending",
            "request": "Approve config",
            "evidence": "",
            "updatedAt": null
        });
        let mut value = crate::model::tests::document(vec![first]);
        value["summary"]["confirmationRequired"] = json!(1);
        let document = TasksFile::from_value(value).unwrap();
        let evaluation = evaluate(&document);
        assert!(evaluation.ready_tasks.is_empty());
        assert!(
            evaluation.blocked_tasks[0]
                .reason
                .contains("confirmation pending")
        );
    }

    #[test]
    fn legacy_batch_simulates_dependency_chain() {
        let first = task("T001");
        let mut second = task("T002");
        second["dependsOn"] = json!(["T001"]);
        let mut third = task("T003");
        third["dependsOn"] = json!(["T002"]);
        let document = parsed(vec![first, second, third]);
        let batch = build_batch_queue(&document);
        assert_eq!(
            batch
                .queue
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            ["T001", "T002", "T003"]
        );
        assert!(batch.blockers.is_empty());
    }
}
