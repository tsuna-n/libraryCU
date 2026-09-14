use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};

use crate::security;

const MAX_HISTORY_BYTES: u64 = 256 * 1024;
const MAX_MESSAGES: usize = 12;
const MAX_CHARS: usize = 16_000;

pub fn history_path() -> PathBuf {
    let base = if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".local/share")
    } else {
        PathBuf::from(".local/share")
    };
    base.join("lbc/history/default.json")
}

pub fn load() -> Result<Vec<String>> {
    let path = history_path();
    crate::security::files::reject_symlinks(&path)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        bail!("refusing to read symlinked history file");
    }
    if fs::metadata(&path)?.len() > MAX_HISTORY_BYTES {
        bail!("history file is larger than 256 KB");
    }
    let content = crate::security::files::read_text(&path, MAX_HISTORY_BYTES)
        .with_context(|| format!("failed to read history at {}", path.display()))?;
    let mut messages: Vec<String> =
        serde_json::from_str(&content).context("invalid persistent history")?;
    bound(&mut messages);
    Ok(messages
        .into_iter()
        .map(|message| security::redact_sensitive(&message))
        .collect())
}

pub fn save(messages: &[String]) -> Result<PathBuf> {
    let path = history_path();
    let parent = path.parent().context("history path has no parent")?;
    let _lock = crate::security::storage::lock_exclusive(&parent.join(".history.lock"))?;
    crate::security::files::reject_symlinks(&path)?;
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(parent)?.file_type().is_symlink() {
        bail!("refusing to use symlinked history directory");
    }
    let mut messages: Vec<_> = messages
        .iter()
        .map(|message| security::redact_sensitive(message))
        .collect();
    bound(&mut messages);
    let encoded = serde_json::to_vec_pretty(&messages)?;
    crate::security::storage::atomic_replace(&path, &encoded, true)?;
    Ok(path)
}

pub fn clear() -> Result<bool> {
    let path = history_path();
    let parent = path.parent().context("history path has no parent")?;
    let _lock = crate::security::storage::lock_exclusive(&parent.join(".history.lock"))?;
    crate::security::files::reject_symlinks(&path)?;
    if !path.exists() {
        return Ok(false);
    }
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        bail!("refusing to remove symlinked history file");
    }
    fs::remove_file(&path)
        .with_context(|| format!("failed to clear history at {}", path.display()))?;
    Ok(true)
}

fn bound(messages: &mut Vec<String>) {
    while messages.len() > MAX_MESSAGES
        || messages.iter().map(String::len).sum::<usize>() > MAX_CHARS
    {
        messages.remove(0);
    }
}
