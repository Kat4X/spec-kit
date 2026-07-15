use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Component, Path},
};

use serde::Serialize;
use serde_json::Value;

use crate::{
    error::AppError,
    model::{Task, TasksFile},
    scheduler::{TaskSummary, UnitMode, evaluate, ready_unit, selected_tasks},
    workspace::{CompactWorkspaceReport, evaluate_workspace},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkPacket {
    pub packet_version: u8,
    pub tasks_schema_version: u64,
    pub change: String,
    pub execution_first_task: String,
    pub workspace: CompactWorkspaceReport,
    pub mode: UnitMode,
    pub revision: String,
    pub tasks: Vec<Value>,
    pub context: PacketContext,
    pub blockers: Vec<TaskSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketContext {
    pub requirements: Vec<RequirementExcerpt>,
    pub missing_refs: Vec<String>,
    pub plan: Vec<PlanExcerpt>,
    pub scope: SourceText,
    pub files: FileContext,
    pub checks: Vec<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RequirementExcerpt {
    pub source: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlanExcerpt {
    pub source: String,
    pub heading: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceText {
    pub source: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FileContext {
    pub read: Vec<String>,
    pub edit: Vec<String>,
    pub create: Vec<String>,
    pub delete: Vec<String>,
}

pub fn build_packet(
    workspace: &Path,
    tasks_bytes: &[u8],
    document: &TasksFile,
    explicit_task_id: Option<&str>,
) -> Result<WorkPacket, AppError> {
    let unit = ready_unit(document, explicit_task_id)?;
    let selected = selected_tasks(document, &unit)?;
    let spec = read_artifact(workspace, &document.spec)?;
    let plan = read_artifact(workspace, &document.plan)?;
    let scope = read_artifact(workspace, &document.scope)?;
    let (requirements, missing_refs) = requirement_context(&spec, &document.spec, &selected);
    let files = file_context(&selected);
    let plan = plan_context(&plan, &document.plan, &selected, &files);
    let checks = check_context(&selected);
    let evaluation = evaluate(document);
    let report = evaluate_workspace(workspace);

    Ok(WorkPacket {
        packet_version: 1,
        tasks_schema_version: document.version,
        change: document.change.clone(),
        execution_first_task: document.first_task.clone(),
        workspace: report.compact(),
        mode: unit.mode,
        revision: revision(tasks_bytes),
        tasks: selected.iter().map(|task| task.raw.clone()).collect(),
        context: PacketContext {
            requirements,
            missing_refs,
            plan,
            scope: SourceText {
                source: document.scope.clone(),
                text: scope,
            },
            files,
            checks,
        },
        blockers: evaluation.blocked_tasks,
    })
}

pub fn revision(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn requirement_context(
    markdown: &str,
    source: &str,
    tasks: &[&Task],
) -> (Vec<RequirementExcerpt>, Vec<String>) {
    let refs = unique_strings(tasks.iter().flat_map(|task| task.refs.iter()));
    let mut requirements = Vec::new();
    let mut missing = Vec::new();
    for reference in refs {
        if let Some(text) = find_reference(markdown, &reference) {
            requirements.push(RequirementExcerpt {
                source: source.into(),
                reference,
                text,
            });
        } else {
            missing.push(reference);
        }
    }
    (requirements, missing)
}

fn find_reference(markdown: &str, reference: &str) -> Option<String> {
    let lines: Vec<_> = markdown.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(level) = heading_level(trimmed) {
            if contains_reference(trimmed, reference) {
                let end = lines
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .find(|(_, candidate)| {
                        heading_level(candidate.trim()).is_some_and(|next| next <= level)
                    })
                    .map(|(next, _)| next)
                    .unwrap_or(lines.len());
                return Some(lines[index..end].join("\n").trim().to_owned());
            }
        } else if is_reference_bullet(trimmed, reference) {
            return Some(trimmed.to_owned());
        }
    }
    None
}

fn plan_context(
    markdown: &str,
    source: &str,
    tasks: &[&Task],
    files: &FileContext,
) -> Vec<PlanExcerpt> {
    let refs = unique_strings(tasks.iter().flat_map(|task| task.refs.iter()));
    let paths = files
        .read
        .iter()
        .chain(files.edit.iter())
        .chain(files.create.iter())
        .chain(files.delete.iter())
        .cloned()
        .collect::<Vec<_>>();
    top_level_sections(markdown)
        .into_iter()
        .filter(|(heading, text)| {
            matches!(
                heading.as_str(),
                "Краткое описание" | "Архитектура / поток" | "API / интерфейсы"
            ) || refs.iter().any(|reference| text.contains(reference))
                || paths
                    .iter()
                    .filter(|path| !path.is_empty())
                    .any(|path| text.contains(path))
        })
        .map(|(heading, text)| PlanExcerpt {
            source: source.into(),
            heading,
            text,
        })
        .collect()
}

fn top_level_sections(markdown: &str) -> Vec<(String, String)> {
    let lines: Vec<_> = markdown.lines().collect();
    let starts: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            (heading_level(trimmed) == Some(2))
                .then(|| (index, trimmed.trim_start_matches('#').trim().to_owned()))
        })
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(position, (start, heading))| {
            let end = starts
                .get(position + 1)
                .map(|(index, _)| *index)
                .unwrap_or(lines.len());
            (
                heading.clone(),
                lines[*start..end].join("\n").trim().to_owned(),
            )
        })
        .collect()
}

fn file_context(tasks: &[&Task]) -> FileContext {
    FileContext {
        read: file_action(tasks, "read"),
        edit: file_action(tasks, "edit"),
        create: file_action(tasks, "create"),
        delete: file_action(tasks, "delete"),
    }
}

fn file_action(tasks: &[&Task], action: &str) -> Vec<String> {
    unique_strings(tasks.iter().flat_map(|task| {
        task.files
            .get(action)
            .into_iter()
            .flat_map(|paths| paths.iter())
    }))
}

fn check_context(tasks: &[&Task]) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    for check in tasks.iter().flat_map(|task| task.checks.iter()) {
        let key = serde_json::to_string(check).unwrap_or_default();
        if seen.insert(key) {
            checks.push(check.clone());
        }
    }
    checks
}

fn unique_strings<'a>(values: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for value in values {
        if seen.insert(value.as_str()) {
            output.push(value.clone());
        }
    }
    output
}

fn read_artifact(workspace: &Path, relative: &str) -> Result<String, AppError> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::Contract(format!(
            "artifact path escapes workspace: {relative}"
        )));
    }
    let workspace = fs::canonicalize(workspace)?;
    let path = fs::canonicalize(workspace.join(relative_path)).map_err(|error| {
        AppError::Io(std::io::Error::new(
            error.kind(),
            format!("artifact {relative}: {error}"),
        ))
    })?;
    if !path.starts_with(&workspace) {
        return Err(AppError::Contract(format!(
            "artifact path escapes workspace: {relative}"
        )));
    }
    fs::read_to_string(path).map_err(AppError::from)
}

fn heading_level(line: &str) -> Option<usize> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (hashes > 0 && line.as_bytes().get(hashes) == Some(&b' ')).then_some(hashes)
}

fn contains_reference(line: &str, reference: &str) -> bool {
    line.match_indices(reference).any(|(index, _)| {
        let before = line[..index].chars().next_back();
        let after = line[index + reference.len()..].chars().next();
        !before.is_some_and(is_ref_character) && !after.is_some_and(is_ref_character)
    })
}

fn is_reference_bullet(line: &str, reference: &str) -> bool {
    let line = line
        .strip_prefix("- [ ] ")
        .or_else(|| line.strip_prefix("- [x] "))
        .or_else(|| line.strip_prefix("- "))
        .unwrap_or(line);
    line.starts_with(reference)
        && line[reference.len()..]
            .chars()
            .next()
            .is_none_or(|character| !is_ref_character(character))
}

fn is_ref_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '-' | '_')
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;

    use crate::model::{TasksFile, tests::task};

    use super::{build_packet, revision};

    fn fixture(mut tasks: Vec<serde_json::Value>) -> (TempDir, Vec<u8>, TasksFile) {
        let directory = TempDir::new().unwrap();
        let path = directory.path();
        fs::write(
            path.join("spec.md"),
            "# Spec\n\n### Case-1: Selected\n\nSELECTED_MARKER\n\n### Case-2: Other\n\nUNRELATED_MARKER\n",
        )
        .unwrap();
        fs::write(
            path.join("plan.md"),
            "# Plan\n\n## Краткое описание\n\nBaseline.\n\n## Архитектура / поток\n\nFlow.\n\n## Несвязанное\n\nUNRELATED_PLAN_MARKER\n",
        )
        .unwrap();
        fs::write(path.join("scope.md"), "# Scope\n\nAllowed files.\n").unwrap();
        fs::write(path.join("verification.md"), "# Verification\n").unwrap();
        for (index, task) in tasks.iter_mut().enumerate() {
            task["refs"] = json!([format!("Case-{}", index + 1)]);
        }
        let value = crate::model::tests::document(tasks);
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(path.join("tasks.json"), &bytes).unwrap();
        let document = TasksFile::from_bytes(&bytes).unwrap();
        (directory, bytes, document)
    }

    #[test]
    fn revision_is_deterministic_and_content_sensitive() {
        assert_eq!(revision(b"abc"), revision(b"abc"));
        assert_ne!(revision(b"abc"), revision(b"abd"));
        assert!(revision(b"abc").starts_with("fnv1a64:"));
    }

    #[test]
    fn single_packet_excludes_unrelated_task_and_context() {
        let (directory, bytes, document) = fixture(vec![task("T001"), task("T002")]);
        let packet = build_packet(directory.path(), &bytes, &document, None).unwrap();
        assert_eq!(packet.tasks.len(), 1);
        assert_eq!(packet.tasks[0]["id"], "T001");
        assert_eq!(packet.context.requirements[0].reference, "Case-1");
        let serialized = serde_json::to_string(&packet).unwrap();
        assert!(serialized.contains("SELECTED_MARKER"));
        assert!(!serialized.contains("UNRELATED_MARKER"));
        assert!(!serialized.contains("UNRELATED_PLAN_MARKER"));
    }

    #[test]
    fn parallel_packet_contains_the_ready_prefix() {
        let mut first = task("T001");
        first["parallel"] = json!(true);
        let mut second = task("T002");
        second["parallel"] = json!(true);
        let (directory, bytes, document) = fixture(vec![first, second]);
        let packet = build_packet(directory.path(), &bytes, &document, None).unwrap();
        assert_eq!(packet.mode, crate::scheduler::UnitMode::Parallel);
        assert_eq!(packet.tasks.len(), 2);
    }

    #[test]
    fn missing_requirement_refs_are_explicit() {
        let (directory, bytes, mut document) = fixture(vec![task("T001")]);
        document.tasks[0].refs = vec!["R-404".into()];
        let packet = build_packet(directory.path(), &bytes, &document, None).unwrap();
        assert!(packet.context.requirements.is_empty());
        assert_eq!(packet.context.missing_refs, ["R-404"]);
    }
}
