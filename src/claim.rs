use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::Serialize;

use crate::{
    error::AppError,
    model::TasksFile,
    packet::{WorkPacket, build_packet, revision},
    scheduler::ready_unit,
    workspace::{Repository, evaluate_workspace},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResult {
    pub dry_run: bool,
    pub claimed: Vec<String>,
    pub previous_revision: String,
    pub revision: String,
    pub packet: WorkPacket,
}

pub fn claim(
    repository: &Repository,
    workspace_value: &Path,
    expected_revision: &str,
    requested_task_ids: &[String],
    dry_run: bool,
) -> Result<ClaimResult, AppError> {
    let workspace = repository.resolve_workspace(workspace_value)?;
    let _lock = LockGuard::acquire(&workspace.join(".workflow-kit.tasks.lock"))?;
    let tasks_path = workspace.join("tasks.json");
    let original_bytes = fs::read(&tasks_path)?;
    let current_revision = revision(&original_bytes);
    if current_revision != expected_revision {
        return Err(AppError::Conflict(format!(
            "stale tasks.json revision: expected {expected_revision}, current {current_revision}"
        )));
    }
    let document = TasksFile::from_bytes(&original_bytes)?;
    let task_ids = resolve_claim_unit(&document, requested_task_ids)?;
    let explicit = (requested_task_ids.len() == 1).then(|| requested_task_ids[0].as_str());
    let mut packet = build_packet(&workspace, &original_bytes, &document, explicit)?;
    let packet_ids: Vec<_> = packet
        .tasks
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect();
    let expected_ids: Vec<_> = task_ids.iter().map(String::as_str).collect();
    if packet_ids != expected_ids {
        return Err(AppError::Conflict(format!(
            "requested claim unit changed: expected {}, current {}",
            expected_ids.join(", "),
            packet_ids.join(", ")
        )));
    }

    let updated = document.with_statuses(&task_ids, "in_progress")?;
    let updated_bytes = updated.to_pretty_bytes()?;
    let updated_revision = revision(&updated_bytes);
    for task in &mut packet.tasks {
        if let Some(task) = task.as_object_mut() {
            task.insert("status".into(), "in_progress".into());
        }
    }
    packet.revision.clone_from(&updated_revision);

    if !dry_run {
        atomic_replace(&tasks_path, &updated_bytes)?;
        packet.workspace = evaluate_workspace(&workspace).compact();
    }

    Ok(ClaimResult {
        dry_run,
        claimed: task_ids,
        previous_revision: current_revision,
        revision: updated_revision,
        packet,
    })
}

fn resolve_claim_unit(
    document: &TasksFile,
    requested_task_ids: &[String],
) -> Result<Vec<String>, AppError> {
    let unique: HashSet<_> = requested_task_ids.iter().collect();
    if unique.len() != requested_task_ids.len() {
        return Err(AppError::Contract("claim task ids must be unique".into()));
    }
    if requested_task_ids.is_empty() {
        return Ok(ready_unit(document, None)?.task_ids);
    }
    if requested_task_ids.len() == 1 {
        return Ok(ready_unit(document, Some(&requested_task_ids[0]))?.task_ids);
    }
    let current = ready_unit(document, None)?.task_ids;
    if current != requested_task_ids {
        return Err(AppError::Conflict(format!(
            "requested tasks are not the current ready unit: requested [{}], current [{}]",
            requested_task_ids.join(", "),
            current.join(", ")
        )));
    }
    Ok(current)
}

fn atomic_replace(target: &Path, contents: &[u8]) -> Result<(), AppError> {
    let parent = target
        .parent()
        .ok_or_else(|| AppError::Contract("tasks.json has no parent directory".into()))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Internal(error.into()))?
        .as_nanos();
    let temporary = parent.join(format!(".tasks.json.tmp.{}.{stamp}", std::process::id()));
    let result = (|| -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Ok(metadata) = fs::metadata(target) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, target)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

struct LockGuard {
    file: File,
    _path: PathBuf,
}

impl LockGuard {
    fn acquire(path: &Path) -> Result<Self, AppError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.lock_exclusive()?;
        Ok(Self {
            file,
            _path: path.to_path_buf(),
        })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        sync::{Arc, Barrier},
        thread,
    };

    use serde_json::json;
    use tempfile::TempDir;

    use crate::{error::AppError, model::tests::task, packet::revision, workspace::Repository};

    use super::claim;

    fn fixture() -> (TempDir, Repository, std::path::PathBuf, String) {
        let directory = TempDir::new().unwrap();
        let workspace = directory.path().join("ai/specs/2026.07.15_00:00_claim");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            workspace.join("spec.md"),
            "# Spec\n\n### Case-1: Claim\n\nClaim it.\n",
        )
        .unwrap();
        fs::write(
            workspace.join("plan.md"),
            "# Plan\n\n## Краткое описание\n\nClaim plan.\n",
        )
        .unwrap();
        fs::write(workspace.join("scope.md"), "# Scope\n\nAllowed.\n").unwrap();
        fs::write(workspace.join("verification.md"), "# Verification\n").unwrap();
        let mut first = task("T001");
        first["refs"] = json!(["Case-1"]);
        let mut value = crate::model::tests::document(vec![first]);
        value["extension"] = json!({"preserved": true});
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(workspace.join("tasks.json"), &bytes).unwrap();
        let repository = Repository::new(directory.path(), Path::new("ai/specs")).unwrap();
        let token = revision(&bytes);
        (directory, repository, workspace, token)
    }

    #[test]
    fn dry_run_returns_hypothetical_packet_without_writing() {
        let (_directory, repository, workspace, token) = fixture();
        let before = fs::read(workspace.join("tasks.json")).unwrap();
        let result = claim(&repository, &workspace, &token, &[], true).unwrap();
        assert!(result.dry_run);
        assert_eq!(result.claimed, ["T001"]);
        assert_eq!(result.packet.tasks[0]["status"], "in_progress");
        assert_eq!(fs::read(workspace.join("tasks.json")).unwrap(), before);
    }

    #[test]
    fn successful_claim_preserves_unknown_data_and_replaces_atomically() {
        let (_directory, repository, workspace, token) = fixture();
        let result = claim(&repository, &workspace, &token, &[], false).unwrap();
        assert_ne!(result.previous_revision, result.revision);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(workspace.join("tasks.json")).unwrap()).unwrap();
        assert_eq!(value["tasks"][0]["status"], "in_progress");
        assert_eq!(value["extension"]["preserved"], true);
        assert!(!fs::read_dir(&workspace).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tasks.json.tmp")
        }));
    }

    #[test]
    fn stale_revision_is_a_conflict_without_changes() {
        let (_directory, repository, workspace, token) = fixture();
        fs::write(workspace.join("unrelated"), "marker").unwrap();
        let error = claim(&repository, &workspace, "fnv1a64:stale", &[], false).unwrap_err();
        assert!(matches!(error, AppError::Conflict(_)));
        let current = fs::read(workspace.join("tasks.json")).unwrap();
        assert_eq!(revision(&current), token);
    }

    #[test]
    fn concurrent_claims_allow_exactly_one_winner() {
        let (_directory, repository, workspace, token) = fixture();
        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let repository = repository.clone();
                let workspace = workspace.clone();
                let token = token.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    claim(&repository, &workspace, &token, &[], false)
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(AppError::Conflict(_))))
                .count(),
            1
        );
    }
}
