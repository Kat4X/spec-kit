mod claim;
mod cli;
mod error;
mod model;
mod packet;
mod scheduler;
mod workspace;

use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};

use clap::Parser;
use cli::{Cli, Command, WorkspaceStatusArg};
use error::{AppError, ErrorPayload};
use serde::Serialize;
use serde_json::Value;
use workspace::{CompactWorkspaceReport, Repository, WorkspaceReport, WorkspaceStatus};

struct Outcome {
    stdout: String,
    code: u8,
}

impl Outcome {
    fn success(stdout: String) -> Self {
        Self { stdout, code: 0 }
    }

    fn with_code(stdout: String, code: u8) -> Self {
        Self { stdout, code }
    }

    fn json<T: Serialize>(value: &T) -> Result<Self, AppError> {
        Ok(Self::success(to_json(value)?))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListPayload {
    specs_dir: PathBuf,
    items: Vec<WorkspaceReport>,
    summary: BTreeMap<String, usize>,
    hidden_done: usize,
}

#[derive(Serialize)]
struct NextTaskPayload {
    workspace: Option<CompactWorkspaceReport>,
    task: Option<Value>,
}

#[derive(Serialize)]
struct BatchQueuePayload {
    workspace: CompactWorkspaceReport,
    queue: Vec<scheduler::TaskSummary>,
    blockers: Vec<scheduler::TaskSummary>,
}

fn run(cli: &Cli) -> Result<Outcome, AppError> {
    let repository = Repository::new(&cli.root, &cli.specs_dir)?;
    match &cli.command {
        Command::Create { name } => {
            let result = repository.create(name)?;
            if cli.json {
                Outcome::json(&result)
            } else {
                Ok(Outcome::success(result.path.display().to_string()))
            }
        }
        Command::Scaffold { workspace, phase } => {
            let result = repository.scaffold(workspace, *phase)?;
            if cli.json {
                Outcome::json(&result)
            } else {
                let created = if result.created.is_empty() {
                    "none".into()
                } else {
                    result.created.join(", ")
                };
                let existing = if result.existing.is_empty() {
                    "none".into()
                } else {
                    result.existing.join(", ")
                };
                Ok(Outcome::success(format!(
                    "{}: created [{created}], existing [{existing}]",
                    result.path.display()
                )))
            }
        }
        Command::List {
            all,
            statuses,
            max_next,
        } => list_command(&repository, cli.json, *all, statuses, *max_next),
        Command::NextTask { workspace, task_id } => {
            next_task_command(&repository, workspace.as_ref(), task_id.as_deref())
        }
        Command::BatchQueue { workspace } => {
            let workspace = repository.resolve_workspace(workspace)?;
            let report = workspace::evaluate_workspace(&workspace);
            let (queue, blockers) = if report.status == WorkspaceStatus::Broken {
                (Vec::new(), Vec::new())
            } else {
                let (_, _, document) = repository.load_tasks(&workspace)?;
                let batch = scheduler::build_batch_queue(&document);
                (batch.queue, batch.blockers)
            };
            let code = u8::from(queue.is_empty());
            let payload = BatchQueuePayload {
                workspace: report.compact(),
                queue,
                blockers,
            };
            Ok(Outcome::with_code(to_json(&payload)?, code))
        }
        Command::Packet { workspace, task_id } => {
            let (workspace, bytes, document) = repository.load_tasks(workspace)?;
            let packet = packet::build_packet(&workspace, &bytes, &document, task_id.as_deref())?;
            Outcome::json(&packet)
        }
        Command::Claim {
            workspace,
            revision,
            task_ids,
            dry_run,
        } => Outcome::json(&claim::claim(
            &repository,
            workspace,
            revision,
            task_ids,
            *dry_run,
        )?),
        Command::Validate { workspace } => {
            let report = repository.validate(workspace)?;
            let code = if report.status == WorkspaceStatus::Broken {
                3
            } else {
                0
            };
            if cli.json {
                Ok(Outcome::with_code(to_json(&report)?, code))
            } else {
                let mut output = format!("{} — {}", report.path.display(), report.status);
                if !report.problems.is_empty() {
                    output.push_str("\nproblems:\n");
                    for problem in &report.problems {
                        output.push_str(&format!("- {problem}\n"));
                    }
                    output = output.trim_end().into();
                }
                Ok(Outcome::with_code(output, code))
            }
        }
    }
}

fn list_command(
    repository: &Repository,
    as_json: bool,
    all: bool,
    requested_statuses: &[WorkspaceStatusArg],
    max_next: usize,
) -> Result<Outcome, AppError> {
    let mut results = repository.discover()?;
    results.sort_by_key(|report| status_rank(report.status));
    let mut summary = BTreeMap::new();
    for status in all_statuses() {
        summary.insert(
            status.to_string(),
            results
                .iter()
                .filter(|report| report.status == status)
                .count(),
        );
    }
    let hidden_done = if all || !requested_statuses.is_empty() {
        0
    } else {
        results
            .iter()
            .filter(|report| report.status == WorkspaceStatus::Done)
            .count()
    };
    let selected_statuses: Vec<_> = requested_statuses
        .iter()
        .copied()
        .map(status_from_arg)
        .collect();
    let visible: Vec<_> = results
        .into_iter()
        .filter(|report| {
            if !selected_statuses.is_empty() {
                selected_statuses.contains(&report.status)
            } else {
                all || report.status != WorkspaceStatus::Done
            }
        })
        .collect();
    if as_json {
        Outcome::json(&ListPayload {
            specs_dir: repository.specs_dir().to_path_buf(),
            items: visible,
            summary,
            hidden_done,
        })
    } else {
        Ok(Outcome::success(list_text(&visible, hidden_done, max_next)))
    }
}

fn next_task_command(
    repository: &Repository,
    workspace_value: Option<&PathBuf>,
    explicit_task_id: Option<&str>,
) -> Result<Outcome, AppError> {
    let maybe_workspace = if let Some(workspace) = workspace_value {
        let path = repository.resolve_workspace(workspace)?;
        Some((path.clone(), workspace::evaluate_workspace(&path)))
    } else {
        repository
            .discover()?
            .into_iter()
            .find(|report| report.status == WorkspaceStatus::Ready)
            .map(|report| (report.path.clone(), report))
    };

    let mut task = None;
    let compact = if let Some((workspace, report)) = maybe_workspace {
        if report.status != WorkspaceStatus::Broken {
            let (_, _, document) = repository.load_tasks(&workspace)?;
            let selected_id = explicit_task_id
                .map(str::to_owned)
                .or_else(|| report.ready_tasks.first().map(|ready| ready.id.clone()));
            if let Some(selected_id) = selected_id {
                task = document.task(&selected_id).map(|task| task.raw.clone());
            }
        }
        Some(report.compact())
    } else {
        None
    };
    let code = u8::from(task.is_none());
    Ok(Outcome::with_code(
        to_json(&NextTaskPayload {
            workspace: compact,
            task,
        })?,
        code,
    ))
}

fn list_text(results: &[WorkspaceReport], hidden_done: usize, max_next: usize) -> String {
    if results.is_empty() {
        return "No Workflow Kit workspaces found for the selected filters.".into();
    }
    let mut output = String::new();
    for status in all_statuses() {
        let group: Vec<_> = results
            .iter()
            .filter(|report| report.status == status)
            .collect();
        if group.is_empty() {
            continue;
        }
        output.push_str(&format!("{status} ({})\n", group.len()));
        for report in group {
            let progress = if report.progress.total == 0 {
                "no tasks".into()
            } else {
                format!("{}/{} done", report.progress.done, report.progress.total)
            };
            output.push_str(&format!("- {} — {progress}\n", report.name));
            let tasks: &[scheduler::TaskSummary] = if status == WorkspaceStatus::Ready {
                &report.ready_tasks
            } else if status == WorkspaceStatus::Blocked {
                &report.blocked_tasks
            } else {
                &[]
            };
            for task in tasks.iter().take(max_next) {
                let reason = if task.reason.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", task.reason)
                };
                output.push_str(&format!(
                    "  {} {} — {}{reason}\n",
                    task.id, task.priority, task.title
                ));
            }
            if !report.missing_files.is_empty() {
                output.push_str(&format!("  missing: {}\n", report.missing_files.join(", ")));
            }
            if !report.problems.is_empty() {
                output.push_str(&format!("  problems: {}\n", report.problems.join("; ")));
            }
        }
        output.push('\n');
    }
    if hidden_done > 0 {
        output.push_str(&format!("DONE hidden: {hidden_done} (--all to show)\n"));
    }
    output.trim_end().into()
}

fn status_from_arg(status: WorkspaceStatusArg) -> WorkspaceStatus {
    match status {
        WorkspaceStatusArg::Ready => WorkspaceStatus::Ready,
        WorkspaceStatusArg::Blocked => WorkspaceStatus::Blocked,
        WorkspaceStatusArg::NeedsTasks => WorkspaceStatus::NeedsTasks,
        WorkspaceStatusArg::NeedsPlan => WorkspaceStatus::NeedsPlan,
        WorkspaceStatusArg::Legacy => WorkspaceStatus::Legacy,
        WorkspaceStatusArg::Broken => WorkspaceStatus::Broken,
        WorkspaceStatusArg::Done => WorkspaceStatus::Done,
    }
}

fn all_statuses() -> [WorkspaceStatus; 7] {
    [
        WorkspaceStatus::Ready,
        WorkspaceStatus::Blocked,
        WorkspaceStatus::NeedsTasks,
        WorkspaceStatus::NeedsPlan,
        WorkspaceStatus::Legacy,
        WorkspaceStatus::Broken,
        WorkspaceStatus::Done,
    ]
}

fn status_rank(status: WorkspaceStatus) -> usize {
    all_statuses()
        .iter()
        .position(|candidate| *candidate == status)
        .expect("known status")
}

fn to_json<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(|error| AppError::Internal(error.into()))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let machine_errors =
        cli.json || matches!(&cli.command, Command::Packet { .. } | Command::Claim { .. });
    match run(&cli) {
        Ok(outcome) => {
            if !outcome.stdout.is_empty() {
                println!("{}", outcome.stdout);
            }
            ExitCode::from(outcome.code)
        }
        Err(error) => {
            if machine_errors {
                match serde_json::to_string(&ErrorPayload::from(&error)) {
                    Ok(payload) => eprintln!("{payload}"),
                    Err(json_error) => eprintln!("internal: {json_error}"),
                }
            } else {
                eprintln!("{}: {error}", error.kind());
            }
            error.exit_code()
        }
    }
}
