use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "workflow-kit",
    version,
    about = "Agent-oriented Spec Kit CLI",
    propagate_version = true
)]
pub struct Cli {
    #[arg(long, global = true, default_value = ".")]
    pub root: PathBuf,

    #[arg(long, global = true, default_value = "ai/specs")]
    pub specs_dir: PathBuf,

    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a change workspace with spec.md.
    Create {
        /// Human-readable change name; Unicode is preserved in the slug.
        name: String,
    },

    /// Scaffold missing artifacts for a workflow phase.
    Scaffold {
        workspace: PathBuf,
        #[arg(long, value_enum)]
        phase: ScaffoldPhase,
    },

    /// List and evaluate Spec Kit workspaces.
    List {
        #[arg(long)]
        all: bool,
        #[arg(long = "status", value_enum)]
        statuses: Vec<WorkspaceStatusArg>,
        #[arg(long, default_value_t = 5)]
        max_next: usize,
    },

    /// Return one legacy next-task payload.
    NextTask {
        workspace: Option<PathBuf>,
        #[arg(long)]
        task_id: Option<String>,
    },

    /// Simulate the legacy dependency-ready queue.
    BatchQueue { workspace: PathBuf },

    /// Return a compact versioned agent work packet.
    Packet {
        workspace: PathBuf,
        #[arg(long)]
        task_id: Option<String>,
    },

    /// Atomically claim a packet's ready task unit.
    Claim {
        workspace: PathBuf,
        #[arg(long)]
        revision: String,
        #[arg(long = "task-id")]
        task_ids: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate workflow artifacts and task contracts.
    Validate { workspace: PathBuf },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ScaffoldPhase {
    Plan,
    Tasks,
    Research,
    DataModel,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum WorkspaceStatusArg {
    #[value(alias = "READY")]
    Ready,
    #[value(alias = "BLOCKED")]
    Blocked,
    #[value(alias = "NEEDS_TASKS")]
    NeedsTasks,
    #[value(alias = "NEEDS_PLAN")]
    NeedsPlan,
    #[value(alias = "LEGACY")]
    Legacy,
    #[value(alias = "BROKEN")]
    Broken,
    #[value(alias = "DONE")]
    Done,
}
