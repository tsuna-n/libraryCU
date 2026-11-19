use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub output: OutputConfig,
    pub scanner: ScannerConfig,
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScannerConfig {
    pub max_file_size_kb: u64,
    pub ignore_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: PathBuf,
    pub found: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
        }
    }
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_file_size_kb: 256,
            ignore_hidden: true,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            mode: "session".to_owned(),
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Some(path) = env::var_os("LBC_CONFIG") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(path).join("lbc/config.toml");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/lbc/config.toml");
    }
    let legacy = PathBuf::from(".config/lbc/config.toml");
    if !matches!(fs::symlink_metadata(&legacy), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        return legacy;
    }
    if let Some(home) = env::var_os("USERPROFILE") {
        return PathBuf::from(home).join(".config/lbc/config.toml");
    }
    legacy
}

pub fn load() -> Result<LoadedConfig> {
    load_from(config_path())
}

pub fn load_from(path: PathBuf) -> Result<LoadedConfig> {
    if !path.exists() {
        return Ok(LoadedConfig {
            config: Config::default(),
            path,
            found: false,
        });
    }

    let content = crate::security::files::read_store_text(&path, 256 * 1024, false)
        .with_context(|| format!("failed to read configuration at {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("invalid configuration at {}", path.display()))?;
    validate(&config)?;

    Ok(LoadedConfig {
        config,
        path,
        found: true,
    })
}

pub fn set_value(key: &str, value: &str) -> Result<PathBuf> {
    set_value_with_preview(key, value, || Ok(()))
}

pub(crate) fn set_value_with_preview(
    key: &str,
    value: &str,
    before_write: impl FnOnce() -> Result<()>,
) -> Result<PathBuf> {
    let loaded = update(
        |config| {
            match key {
                "output.language" => config.output.language = value.to_owned(),
                "scanner.max_file_size_kb" => {
                    config.scanner.max_file_size_kb = value
                        .parse()
                        .with_context(|| format!("{key} must be a positive integer"))?;
                }
                "scanner.ignore_hidden" => {
                    config.scanner.ignore_hidden = value
                        .parse()
                        .with_context(|| format!("{key} must be true or false"))?;
                }
                "memory.mode" => config.memory.mode = value.to_owned(),
                _ => bail!(
                    "unsupported setting {key:?}; supported settings: output.language, scanner.max_file_size_kb, scanner.ignore_hidden, memory.mode"
                ),
            }
            Ok(())
        },
        before_write,
    )?;
    Ok(loaded.path)
}

/// Validate the complete candidate and preserve extension keys under one store lock.
pub(crate) fn update(
    change: impl FnOnce(&mut Config) -> Result<()>,
    before_write: impl FnOnce() -> Result<()>,
) -> Result<LoadedConfig> {
    let path = config_path();
    crate::security::files::reject_symlinks(&path)?;
    let _lock = crate::security::storage::lock_exclusive(&path.with_extension("lock"))?;
    let loaded = load_from(path.clone())?;
    let mut config = loaded.config;
    change(&mut config)?;
    validate(&config)?;
    let mut document = if loaded.found {
        let content = crate::security::files::read_store_text(&path, 256 * 1024, false)?;
        toml::from_str::<toml::Value>(&content).context("invalid configuration")?
    } else {
        toml::Value::Table(Default::default())
    };
    merge(&mut document, toml::Value::try_from(&config)?);
    let encoded = toml::to_string_pretty(&document).context("failed to encode configuration")?;
    anyhow::ensure!(encoded.len() <= 256 * 1024, "configuration exceeds 256 KB");
    before_write()?;
    crate::security::storage::atomic_replace(&path, encoded.as_bytes(), true)
        .with_context(|| format!("failed to write configuration at {}", path.display()))?;
    load_from(path)
}

fn merge(existing: &mut toml::Value, candidate: toml::Value) {
    if let (Some(existing), toml::Value::Table(candidate)) = (existing.as_table_mut(), &candidate) {
        for (key, value) in candidate {
            if let Some(previous) = existing.get_mut(key) {
                merge(previous, value.clone());
            } else {
                existing.insert(key.clone(), value.clone());
            }
        }
    } else {
        *existing = candidate;
    }
}

pub(crate) fn validate(config: &Config) -> Result<()> {
    if !matches!(config.output.language.as_str(), "en" | "th" | "auto") {
        bail!("output.language must be en, th, or auto");
    }
    if config.scanner.max_file_size_kb == 0 {
        bail!("scanner.max_file_size_kb must be greater than zero");
    }
    if !matches!(config.memory.mode.as_str(), "session" | "persistent") {
        bail!("memory.mode must be session or persistent");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temporary_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        env::temp_dir().join(format!("lbc-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn missing_config_uses_defaults() -> Result<()> {
        let loaded = load_from(temporary_path("missing-config"))?;
        assert!(!loaded.found);
        assert_eq!(loaded.config.output.language, "auto");
        assert_eq!(loaded.config.scanner.max_file_size_kb, 256);
        Ok(())
    }

    #[test]
    fn partial_config_inherits_defaults() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("config.toml");
        fs::write(&path, "[output]\nlanguage = \"th\"\n")?;
        let loaded = load_from(path.clone())?;
        assert_eq!(loaded.config.output.language, "th");
        assert!(loaded.config.scanner.ignore_hidden);
        fs::remove_file(path)?;
        Ok(())
    }
}
