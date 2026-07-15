use std::{
    fmt, fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use chrono::Local;
use serde::Serialize;

use crate::{
    cli::ScaffoldPhase,
    error::AppError,
    model::TasksFile,
    scheduler::{Progress, TaskSummary, evaluate},
};

const REQUIRED_FOR_IMPLEMENT: [&str; 5] = [
    "spec.md",
    "plan.md",
    "scope.md",
    "tasks.json",
    "verification.md",
];

#[derive(Clone, Debug)]
pub struct Repository {
    root: PathBuf,
    specs_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResult {
    pub workspace: String,
    pub path: PathBuf,
    pub created: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaffoldResult {
    pub workspace: String,
    pub path: PathBuf,
    pub phase: String,
    pub created: Vec<String>,
    pub existing: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceStatus {
    Ready,
    Blocked,
    NeedsTasks,
    NeedsPlan,
    Legacy,
    Broken,
    Done,
}

impl fmt::Display for WorkspaceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = serde_json::to_value(self).map_err(|_| fmt::Error)?;
        formatter.write_str(value.as_str().ok_or(fmt::Error)?)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceReport {
    pub name: String,
    pub path: PathBuf,
    pub status: WorkspaceStatus,
    pub progress: Progress,
    pub ready_tasks: Vec<TaskSummary>,
    pub blocked_tasks: Vec<TaskSummary>,
    pub missing_files: Vec<String>,
    pub problems: Vec<String>,
    pub command: Option<String>,
}

impl WorkspaceReport {
    fn initial(path: &Path) -> Self {
        Self {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            path: path.to_path_buf(),
            status: WorkspaceStatus::Broken,
            progress: Progress::default(),
            ready_tasks: Vec::new(),
            blocked_tasks: Vec::new(),
            missing_files: Vec::new(),
            problems: Vec::new(),
            command: None,
        }
    }

    pub fn compact(&self) -> CompactWorkspaceReport {
        CompactWorkspaceReport {
            name: self.name.clone(),
            path: self.path.clone(),
            status: self.status,
            progress: self.progress.clone(),
            missing_files: self.missing_files.clone(),
            problems: self.problems.clone(),
            command: self.command.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactWorkspaceReport {
    pub name: String,
    pub path: PathBuf,
    pub status: WorkspaceStatus,
    pub progress: Progress,
    pub missing_files: Vec<String>,
    pub problems: Vec<String>,
    pub command: Option<String>,
}

impl Repository {
    pub fn new(root: &Path, specs_dir: &Path) -> Result<Self, AppError> {
        let root = fs::canonicalize(root).map_err(|error| {
            AppError::Io(std::io::Error::new(
                error.kind(),
                format!("repository root {}: {error}", root.display()),
            ))
        })?;
        let specs_dir = contained_join(&root, specs_dir, "specs directory")?;
        Ok(Self { root, specs_dir })
    }

    pub fn specs_dir(&self) -> &Path {
        &self.specs_dir
    }

    pub fn create(&self, name: &str) -> Result<CreateResult, AppError> {
        let slug = slug(name)?;
        fs::create_dir_all(&self.specs_dir)?;
        ensure_existing_contained(&self.root, &self.specs_dir, "specs directory")?;
        let template =
            self.template(".agents/skills/workflow-kit-specify/assets/SPEC-TEMPLATE.md")?;

        let timestamp = Local::now().format("%Y.%m.%d_%H:%M").to_string();
        let mut suffix = 1;
        let workspace = loop {
            let suffix_text = if suffix == 1 {
                String::new()
            } else {
                format!("-{suffix}")
            };
            let candidate = self
                .specs_dir
                .join(format!("{timestamp}_{slug}{suffix_text}"));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => suffix += 1,
                Err(error) => return Err(error.into()),
            }
        };

        if let Err(error) = copy_new(&template, &workspace.join("spec.md")) {
            let _ = fs::remove_dir_all(&workspace);
            return Err(error);
        }
        Ok(CreateResult {
            workspace: file_name(&workspace),
            path: workspace,
            created: vec!["spec.md".into()],
        })
    }

    pub fn scaffold(
        &self,
        workspace: &Path,
        phase: ScaffoldPhase,
    ) -> Result<ScaffoldResult, AppError> {
        let workspace = self.resolve_workspace(workspace)?;
        let (phase_name, prerequisites, templates): (&str, &[&str], &[(&str, &str)]) = match phase {
            ScaffoldPhase::Plan => (
                "plan",
                &["spec.md"],
                &[
                    (
                        ".agents/skills/workflow-kit-plan/assets/PLAN-TEMPLATE.md",
                        "plan.md",
                    ),
                    (
                        ".agents/skills/workflow-kit-plan/assets/SCOPE-TEMPLATE.md",
                        "scope.md",
                    ),
                ],
            ),
            ScaffoldPhase::Tasks => (
                "tasks",
                &["spec.md", "plan.md", "scope.md"],
                &[
                    (
                        ".agents/skills/workflow-kit-tasks/assets/TASKS-TEMPLATE.json",
                        "tasks.json",
                    ),
                    (
                        ".agents/skills/workflow-kit-tasks/assets/VERIFICATION-TEMPLATE.md",
                        "verification.md",
                    ),
                ],
            ),
            ScaffoldPhase::Research => (
                "research",
                &["spec.md"],
                &[(
                    ".agents/skills/workflow-kit-plan/assets/RESEARCH-TEMPLATE.md",
                    "research.md",
                )],
            ),
            ScaffoldPhase::DataModel => (
                "data-model",
                &["spec.md", "plan.md"],
                &[(
                    ".agents/skills/workflow-kit-plan/assets/DATA-MODEL-TEMPLATE.md",
                    "data-model.md",
                )],
            ),
        };
        let missing: Vec<_> = prerequisites
            .iter()
            .filter(|name| !workspace.join(name).is_file())
            .copied()
            .collect();
        if !missing.is_empty() {
            return Err(AppError::Contract(format!(
                "cannot scaffold {phase_name}; missing prerequisites: {}",
                missing.join(", ")
            )));
        }

        let mut created = Vec::new();
        let mut existing = Vec::new();
        for (template_name, target_name) in templates {
            let target = workspace.join(target_name);
            if target.exists() {
                existing.push((*target_name).to_owned());
                continue;
            }
            let template = self.template(template_name)?;
            copy_new(&template, &target)?;
            created.push((*target_name).to_owned());
        }
        Ok(ScaffoldResult {
            workspace: file_name(&workspace),
            path: workspace,
            phase: phase_name.into(),
            created,
            existing,
        })
    }

    pub fn resolve_workspace(&self, value: &Path) -> Result<PathBuf, AppError> {
        reject_parent_components(value, "workspace path")?;
        let direct = if value.is_absolute() {
            value.to_path_buf()
        } else {
            let named = self.specs_dir.join(value);
            if named.is_dir() {
                named
            } else {
                self.root.join(value)
            }
        };
        let resolved = fs::canonicalize(&direct).map_err(|error| {
            AppError::Io(std::io::Error::new(
                error.kind(),
                format!("workspace {}: {error}", direct.display()),
            ))
        })?;
        ensure_path_starts_with(&self.root, &resolved, "workspace path")?;
        if !resolved.is_dir() {
            return Err(AppError::Contract(format!(
                "workspace is not a directory: {}",
                resolved.display()
            )));
        }
        Ok(resolved)
    }

    pub fn discover(&self) -> Result<Vec<WorkspaceReport>, AppError> {
        if !self.specs_dir.is_dir() {
            return Err(AppError::NoWork(format!(
                "Specs directory not found: {}",
                self.specs_dir.display()
            )));
        }
        ensure_existing_contained(&self.root, &self.specs_dir, "specs directory")?;
        let mut workspaces: Vec<_> = fs::read_dir(&self.specs_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        workspaces.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        Ok(workspaces
            .iter()
            .map(|workspace| evaluate_workspace(workspace))
            .collect())
    }

    pub fn validate(&self, workspace: &Path) -> Result<WorkspaceReport, AppError> {
        let workspace = self.resolve_workspace(workspace)?;
        let mut report = evaluate_workspace(&workspace);
        if report.status == WorkspaceStatus::Broken {
            return Ok(report);
        }
        if workspace.join("tasks.json").is_file() {
            let document = load_tasks(&workspace)?;
            let links = [
                Some(document.spec.as_str()),
                Some(document.plan.as_str()),
                document.research.as_deref(),
                document.data_model.as_deref(),
                Some(document.scope.as_str()),
                Some(document.verification.as_str()),
            ];
            for link in links.into_iter().flatten() {
                if let Err(error) = artifact_path(&workspace, link) {
                    report.problems.push(error.to_string());
                    continue;
                }
                if !workspace.join(link).is_file() {
                    report
                        .problems
                        .push(format!("referenced artifact is missing: {link}"));
                }
            }
        }
        for template in [
            ".agents/skills/workflow-kit-specify/assets/SPEC-TEMPLATE.md",
            ".agents/skills/workflow-kit-plan/assets/PLAN-TEMPLATE.md",
            ".agents/skills/workflow-kit-plan/assets/SCOPE-TEMPLATE.md",
            ".agents/skills/workflow-kit-tasks/assets/TASKS-TEMPLATE.json",
            ".agents/skills/workflow-kit-tasks/assets/VERIFICATION-TEMPLATE.md",
        ] {
            if let Err(error) = self.template(template) {
                report.problems.push(error.to_string());
            }
        }
        if !report.problems.is_empty() {
            report.status = WorkspaceStatus::Broken;
        }
        Ok(report)
    }

    pub fn load_tasks(&self, workspace: &Path) -> Result<(PathBuf, Vec<u8>, TasksFile), AppError> {
        let workspace = self.resolve_workspace(workspace)?;
        let path = workspace.join("tasks.json");
        let bytes = fs::read(&path)?;
        let document = TasksFile::from_bytes(&bytes)?;
        Ok((workspace, bytes, document))
    }

    fn template(&self, relative: &str) -> Result<PathBuf, AppError> {
        let path = contained_join(&self.root, Path::new(relative), "template path")?;
        if !path.is_file() {
            return Err(AppError::Contract(format!(
                "required template not found: {}",
                path.display()
            )));
        }
        ensure_existing_contained(&self.root, &path, "template path")?;
        Ok(path)
    }
}

pub fn evaluate_workspace(workspace: &Path) -> WorkspaceReport {
    let mut report = WorkspaceReport::initial(workspace);
    if !workspace.is_dir() {
        report
            .problems
            .push("workspace directory is missing".into());
        return report;
    }
    let has_spec = workspace.join("spec.md").is_file();
    let has_plan = workspace.join("plan.md").is_file();
    let has_scope = workspace.join("scope.md").is_file();
    let has_tasks = workspace.join("tasks.json").is_file();
    let has_legacy_tasks = workspace.join("tasks.md").is_file();

    if !has_spec {
        report.missing_files.push("spec.md".into());
        report.problems.push("workspace has no spec.md".into());
        return report;
    }
    if !has_tasks {
        if has_legacy_tasks {
            report.status = WorkspaceStatus::Legacy;
            report.missing_files.push("tasks.json".into());
            report.problems.push(
                "legacy tasks.md exists, but tasks.json is required for implementation".into(),
            );
            return report;
        }
        if !has_plan || !has_scope {
            report.status = WorkspaceStatus::NeedsPlan;
            if !has_plan {
                report.missing_files.push("plan.md".into());
            }
            if !has_scope {
                report.missing_files.push("scope.md".into());
            }
            report
                .problems
                .push("plan/scope stage is not complete".into());
            return report;
        }
        report.status = WorkspaceStatus::NeedsTasks;
        report.missing_files.push("tasks.json".into());
        report.problems.push("tasks.json is missing".into());
        return report;
    }

    let missing: Vec<_> = REQUIRED_FOR_IMPLEMENT
        .iter()
        .filter(|name| !workspace.join(name).is_file())
        .map(|name| (*name).to_owned())
        .collect();
    if !missing.is_empty() {
        report.missing_files = missing;
        report
            .problems
            .push("implementation-required workflow files are missing".into());
        return report;
    }

    let document = match load_tasks(workspace) {
        Ok(document) => document,
        Err(error) => {
            report.problems.push(error.to_string());
            return report;
        }
    };
    let evaluation = evaluate(&document);
    report.progress = evaluation.progress;
    report.ready_tasks = evaluation.ready_tasks;
    report.blocked_tasks = evaluation.blocked_tasks;
    if let Some(first) = report.ready_tasks.first() {
        report.status = WorkspaceStatus::Ready;
        report.command = Some(format!(
            "workflow-kit-implement {} {}",
            report.name, first.id
        ));
    } else if report.progress.done < report.progress.total {
        report.status = WorkspaceStatus::Blocked;
    } else {
        report.status = WorkspaceStatus::Done;
    }
    report
}

fn load_tasks(workspace: &Path) -> Result<TasksFile, AppError> {
    TasksFile::from_bytes(&fs::read(workspace.join("tasks.json"))?)
}

fn slug(value: &str) -> Result<String, AppError> {
    let mut output = String::new();
    let mut separator = false;
    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    if output.is_empty() {
        return Err(AppError::Contract(
            "change name is empty after normalization".into(),
        ));
    }
    Ok(output)
}

fn copy_new(source: &Path, target: &Path) -> Result<(), AppError> {
    let contents = fs::read(source)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    file.write_all(&contents)?;
    file.sync_all()?;
    Ok(())
}

fn contained_join(root: &Path, value: &Path, label: &str) -> Result<PathBuf, AppError> {
    reject_parent_components(value, label)?;
    let candidate = if value.is_absolute() {
        value.to_path_buf()
    } else {
        root.join(value)
    };
    ensure_path_starts_with(root, &candidate, label)?;
    Ok(candidate)
}

fn reject_parent_components(path: &Path, label: &str) -> Result<(), AppError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::Contract(format!(
            "{label} cannot contain parent traversal: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_existing_contained(root: &Path, path: &Path, label: &str) -> Result<(), AppError> {
    let canonical = fs::canonicalize(path)?;
    ensure_path_starts_with(root, &canonical, label)
}

fn ensure_path_starts_with(root: &Path, path: &Path, label: &str) -> Result<(), AppError> {
    if !path.starts_with(root) {
        return Err(AppError::Contract(format!(
            "{label} escapes repository root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn artifact_path(workspace: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let value = Path::new(relative);
    if value.is_absolute() {
        return Err(AppError::Contract(format!(
            "artifact path must be relative: {relative}"
        )));
    }
    reject_parent_components(value, "artifact path")?;
    Ok(workspace.join(value))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{Repository, WorkspaceStatus, evaluate_workspace, slug};
    use crate::{cli::ScaffoldPhase, model::tests::task};

    fn repository() -> (TempDir, Repository) {
        let directory = TempDir::new().unwrap();
        for (relative, contents) in [
            (
                ".agents/skills/workflow-kit-specify/assets/SPEC-TEMPLATE.md",
                "# spec template\n",
            ),
            (
                ".agents/skills/workflow-kit-plan/assets/PLAN-TEMPLATE.md",
                "# plan template\n",
            ),
            (
                ".agents/skills/workflow-kit-plan/assets/SCOPE-TEMPLATE.md",
                "# scope template\n",
            ),
            (
                ".agents/skills/workflow-kit-plan/assets/RESEARCH-TEMPLATE.md",
                "# research template\n",
            ),
            (
                ".agents/skills/workflow-kit-plan/assets/DATA-MODEL-TEMPLATE.md",
                "# data model template\n",
            ),
            (
                ".agents/skills/workflow-kit-tasks/assets/TASKS-TEMPLATE.json",
                "{}\n",
            ),
            (
                ".agents/skills/workflow-kit-tasks/assets/VERIFICATION-TEMPLATE.md",
                "# verification template\n",
            ),
        ] {
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        let repository =
            Repository::new(directory.path(), std::path::Path::new("ai/specs")).unwrap();
        (directory, repository)
    }

    #[test]
    fn slug_preserves_unicode_and_rejects_empty_names() {
        assert_eq!(slug("CSV экспорт задач").unwrap(), "csv-экспорт-задач");
        assert!(slug("---").is_err());
    }

    #[test]
    fn create_is_unique_and_copies_canonical_spec() {
        let (_directory, repository) = repository();
        let first = repository.create("Экспорт задач").unwrap();
        let second = repository.create("Экспорт задач").unwrap();
        assert_ne!(first.path, second.path);
        assert!(first.workspace.contains("экспорт-задач"));
        assert_eq!(
            fs::read_to_string(first.path.join("spec.md")).unwrap(),
            "# spec template\n"
        );
    }

    #[test]
    fn scaffold_is_idempotent_and_never_overwrites() {
        let (_directory, repository) = repository();
        let created = repository.create("Scaffold test").unwrap();
        fs::write(created.path.join("plan.md"), "custom plan\n").unwrap();
        let result = repository
            .scaffold(&created.path, ScaffoldPhase::Plan)
            .unwrap();
        assert_eq!(result.created, ["scope.md"]);
        assert_eq!(result.existing, ["plan.md"]);
        assert_eq!(
            fs::read_to_string(created.path.join("plan.md")).unwrap(),
            "custom plan\n"
        );
    }

    #[test]
    fn workspace_path_cannot_escape_root() {
        let (_directory, repository) = repository();
        let error = repository
            .resolve_workspace(std::path::Path::new("../outside"))
            .unwrap_err();
        assert!(error.to_string().contains("parent traversal"));
    }

    #[test]
    fn evaluates_ready_workspace() {
        let (_directory, repository) = repository();
        let created = repository.create("Ready fixture").unwrap();
        for name in ["plan.md", "scope.md", "verification.md"] {
            fs::write(created.path.join(name), name).unwrap();
        }
        let document = crate::model::tests::document(vec![task("T001")]);
        fs::write(
            created.path.join("tasks.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        let report = evaluate_workspace(&created.path);
        assert_eq!(report.status, WorkspaceStatus::Ready);
        assert_eq!(report.ready_tasks[0].id, "T001");
    }
}
