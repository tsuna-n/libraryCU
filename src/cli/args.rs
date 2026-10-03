use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "lbc",
    version,
    about = "libraryCube - local-first developer knowledge engine",
    after_help = "Save: lbc learn / add\nFind: lbc search\nReuse: lbc ask / explain\nAI is optional. Core knowledge workflows work offline.",
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
    /// Capture a solved problem as portable local Markdown
    Learn(Box<LearnArgs>),
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
    /// Optional: start a bounded interactive knowledge Q&A session
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
    /// Experimental: generate a minimal AI patch grounded in local knowledge
    Fix(FixArgs),
    /// Experimental: apply a saved proposal without contacting AI
    ApplyProposal(ApplyProposalArgs),
    /// Experimental support: restore a recorded fix if its target still matches
    Rollback(RollbackArgs),
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
pub struct LearnArgs {
    #[arg(long)]
    pub problem: Option<String>,
    #[arg(long)]
    pub cause: Option<String>,
    #[arg(long)]
    pub solution: Option<String>,
    /// A historical check to record; this command never executes it
    #[arg(long)]
    pub verification: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub tool: Option<String>,
    #[arg(long)]
    pub framework: Option<String>,
    #[arg(long)]
    pub error_code: Option<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub symptoms: Option<String>,
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long = "reference")]
    pub references: Vec<String>,
    /// Save in PATH/.lbc/knowledge (default: current directory)
    #[arg(long, conflicts_with = "user")]
    pub project: Option<PathBuf>,
    /// Save in your personal notes store
    #[arg(long)]
    pub user: bool,
    /// Save supplied fields without prompts; requires --problem and --solution
    #[arg(long, requires_all = ["problem", "solution"])]
    pub yes: bool,
    /// Machine-readable output for noninteractive capture
    #[arg(long, requires = "yes")]
    pub json: bool,
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
pub struct FixArgs {
    /// Error log file
    #[arg(value_name = "FILE", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// Read error output from stdin
    #[arg(long)]
    pub stdin: bool,
    /// Request an AI patch using bounded local evidence
    #[arg(long)]
    pub ai: bool,
    /// Apply the validated patch after generating it
    #[arg(long, requires = "ai")]
    pub apply: bool,
    /// Save the validated preview privately for later apply-proposal
    #[arg(long, requires = "ai", conflicts_with = "apply")]
    pub save_proposal: bool,
    /// Run an explicit command after application; failure attempts single-file rollback
    #[arg(long, value_name = "COMMAND", requires = "apply")]
    pub verify: Option<String>,
    /// Verification deadline in seconds (includes output collection)
    #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..=3600), requires = "verify")]
    pub verify_timeout: u64,
    /// Output machine-readable JSON
    #[arg(long)]
    pub json: bool,
    /// Project path containing the file reported by the diagnostic
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct RollbackArgs {
    /// ID printed by fix --apply
    pub id: String,
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ApplyProposalArgs {
    pub id: String,
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long, value_name = "COMMAND")]
    pub verify: Option<String>,
    #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..=3600), requires = "verify")]
    pub verify_timeout: u64,
    #[arg(long)]
    pub json: bool,
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
    /// Explicitly contact the configured provider's models endpoint (no generation)
    #[arg(long)]
    pub connectivity: bool,
    /// Connectivity deadline in seconds
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..=45), requires = "connectivity")]
    pub connectivity_timeout: u64,
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
