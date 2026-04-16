use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod validate;

const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: String,
    pub remote_dir: String,
    pub shortcut: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file missing `version` field or has unsupported version")]
    UnversionedOrUnsupported,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let raw = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    match v.get("version").and_then(|x| x.as_u64()) {
        Some(n) if n as u32 == CURRENT_VERSION => Ok(serde_json::from_value(v)?),
        _ => Err(ConfigError::UnversionedOrUnsupported),
    }
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn config_file(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join("clipship").join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "example.com".into();
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn load_rejects_file_without_version_field() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, r#"{"host":"x","port":22}"#).unwrap();
        let err = load(&path).unwrap_err();
        assert!(matches!(err, ConfigError::UnversionedOrUnsupported));
    }

    #[test]
    fn load_rejects_future_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, r#"{"version":2}"#).unwrap();
        let err = load(&path).unwrap_err();
        assert!(matches!(err, ConfigError::UnversionedOrUnsupported));
    }
}
