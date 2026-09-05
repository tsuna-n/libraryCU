use anyhow::Result;

use crate::{cli::args::HistoryCommand, history};

pub fn run(command: HistoryCommand) -> Result<()> {
    match command {
        HistoryCommand::Show { json } => {
            let path = history::history_path();
            let messages = history::load()?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"path": path, "exists": path.exists(), "messages": messages.len()})
                );
            } else {
                println!(
                    "Persistent history\n  Path: {}\n  Exists: {}\n  Messages: {}",
                    path.display(),
                    path.exists(),
                    messages.len()
                );
            }
        }
        HistoryCommand::Clear { json } => {
            let path = history::history_path();
            let removed = history::clear()?;
            if json {
                println!("{}", serde_json::json!({"path": path, "removed": removed}));
            } else if removed {
                println!("Cleared persistent history at {}", path.display());
            } else {
                println!("No persistent history existed at {}", path.display());
            }
        }
    }
    Ok(())
}
