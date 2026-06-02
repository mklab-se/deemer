//! Tool configuration.
//!
//! A minimal, reusable starting point: a YAML config stored in the platform
//! config directory (`~/.config/deemer/config.yaml` on Linux/macOS). No
//! command uses it yet — it's here so a new tool has somewhere obvious to grow
//! its settings. Add fields to [`Config`] and they round-trip automatically.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Persisted configuration for the tool.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Example setting. Replace with your tool's real configuration.
    pub example: Option<String>,
}

impl Config {
    /// Return the path to the config file, creating no files.
    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir().ok_or(Error::NoConfigDir)?;
        Ok(dir.join("deemer").join("config.yaml"))
    }

    /// Load the config from disk. Returns [`Config::default`] if no file exists yet.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let yaml = std::fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&yaml)?)
    }

    /// Save the config to disk, creating the parent directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_yaml() {
        let cfg = Config {
            example: Some("hello".to_string()),
        };
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.example.as_deref(), Some("hello"));
    }

    #[test]
    fn empty_yaml_uses_defaults() {
        let parsed: Config = serde_yaml::from_str("{}").unwrap();
        assert!(parsed.example.is_none());
    }
}
