use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "lbc",
    version,
    about = "libraryCube - terminal knowledge library",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Add a Markdown entry to your knowledge library
    Add(AddArgs),
    /// List effective knowledge entries
    List(ListArgs),
    /// Show a complete knowledge entry
    Inspect(InspectArgs),
    /// Replace the body of a writable knowledge entry
    Edit(EditArgs),
    /// Validate and rebuild the in-memory retrieval index
    Index(IndexArgs),
    /// Answer a question from retrieved local knowledge
    Ask(AskArgs),
    /// Start a bounded interactive question session
    Chat(ChatArgs),
    /// Inspect or clear explicitly persisted chat history
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
    /// Inspect the current project
    Scan(ScanArgs),
    /// Explain compiler or runtime errors
    Explain(ExplainArgs),
    /// Search local technical knowledge
    Search(SearchArgs),
    /// View or modify LBC configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Check the LBC environment
    Doctor(DoctorArgs),
    /// Manage installable knowledge packages
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub title: String,
    #[arg(long, default_value = "note", value_parser = ["note", "concept", "troubleshooting"])]
    pub kind: String,
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "stdin",
        required_unless_present = "stdin"
    )]
    pub file: Option<PathBuf>,
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
    /// Store in PATH/.lbc/knowledge instead of the user notes store
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    pub source_id: String,
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    pub source_id: String,
    #[arg(long, value_name = "FILE")]
    pub file: Option<PathBuf>,
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub r#override: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AskArgs {
    pub question: String,
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub ai: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ChatArgs {
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub ai: bool,
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    /// Show whether persistent history exists and where it is stored
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Delete the explicit persistent history file
    Clear {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Project path to inspect
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    /// Display the project file tree
    #[arg(long)]
    pub tree: bool,
    /// Output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// Error log file
    #[arg(value_name = "FILE", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// Read error from stdin
    #[arg(long)]
    pub stdin: bool,
    /// Show detailed explanation
    #[arg(long)]
    pub verbose: bool,
    /// Output machine-readable JSON
    #[arg(long)]
    pub json: bool,
    /// Extend the deterministic analysis with the configured AI provider
    #[arg(long)]
    pub ai: bool,
    /// Project path used for contextual evidence
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Error code or technical keywords
    pub query: String,
    /// Project path containing optional knowledge documents
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    /// Output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the effective configuration
    Show {
        /// Output machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Set a supported configuration value
    Set {
        /// Setting name, for example scanner.max_file_size_kb
        key: String,
        /// New setting value
        value: String,
    },
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeCommand {
    /// Install a knowledge package from a local directory
    Install {
        /// Directory containing package.toml and markdown documents
        source: PathBuf,
    },
    /// List installed knowledge packages
    List,
    /// Remove an installed knowledge package
    Remove {
        /// Package name shown by `lbc knowledge list`
        name: String,
    },
}
