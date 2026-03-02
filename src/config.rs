use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub commands: BTreeMap<String, CommandConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandConfig {
    pub run: String,
}

impl Config {
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config = serde_yaml::from_str::<Config>(&contents).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    pub fn default_path() -> Result<PathBuf, ConfigError> {
        if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(path).join("hatch").join("hatch.yaml"));
        }

        let home = env::var_os("HOME").ok_or(ConfigError::MissingConfigHome)?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("hatch")
            .join("hatch.yaml"))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.commands.is_empty() {
            return Err(ConfigError::Invalid(
                "config must define at least one command".to_string(),
            ));
        }

        for (name, command) in &self.commands {
            if name.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "command names must not be blank".to_string(),
                ));
            }

            if command.run.trim().is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "command `{name}` must define a non-empty run value"
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    MissingConfigHome,
    Read { path: PathBuf, source: std::io::Error },
    Parse(serde_yaml::Error),
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingConfigHome => {
                write!(f, "unable to resolve config directory from XDG_CONFIG_HOME or HOME")
            }
            ConfigError::Read { path, source } => {
                write!(f, "failed to read config from {}: {source}", path.display())
            }
            ConfigError::Parse(source) => write!(f, "failed to parse config: {source}"),
            ConfigError::Invalid(message) => write!(f, "invalid config: {message}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse(source) => Some(source),
            ConfigError::MissingConfigHome | ConfigError::Invalid(_) => None,
        }
    }
}
