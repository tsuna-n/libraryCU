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
    if let Some(path) = env::var_os("LCU_CONFIG") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(path).join("lcu/config.toml");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/lcu/config.toml");
    }
    PathBuf::from(".config/lcu/config.toml")
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

    let content = fs::read_to_string(&path)
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
    let path = config_path();
    let mut config = load_from(path.clone())?.config;

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
    validate(&config)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let encoded = toml::to_string_pretty(&config).context("failed to encode configuration")?;
    fs::write(&path, encoded)
        .with_context(|| format!("failed to write configuration at {}", path.display()))?;
    Ok(path)
}

fn validate(config: &Config) -> Result<()> {
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
        env::temp_dir().join(format!("lcu-{name}-{}-{nonce}", std::process::id()))
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
        let path = temporary_path("partial-config");
        fs::write(&path, "[output]\nlanguage = \"th\"\n")?;
        let loaded = load_from(path.clone())?;
        assert_eq!(loaded.config.output.language, "th");
        assert!(loaded.config.scanner.ignore_hidden);
        fs::remove_file(path)?;
        Ok(())
    }
}
