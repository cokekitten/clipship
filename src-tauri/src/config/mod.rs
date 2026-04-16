use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod validate;

const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    Ssh,
    #[default]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub mode: UploadMode,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: String,
    pub remote_dir: String,
    pub shortcut: String,
    #[serde(default)]
    pub shortcut_double_tap: bool,
    #[serde(default)]
    pub auto_cleanup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            mode: UploadMode::Local,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
            auto_cleanup: false,
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
    match v.get("version") {
        Some(version)
            if version.as_u64() == Some(CURRENT_VERSION as u64)
                && version.as_i64() == Some(CURRENT_VERSION as i64) =>
        {
            Ok(serde_json::from_value(v)?)
        }
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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{field}: {err}")]
pub struct ValidationError {
    pub field: &'static str,
    pub err: validate::FieldError,
}

impl Config {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.mode == UploadMode::Ssh {
            validate::host(&self.host).map_err(|e| ValidationError { field: "host", err: e })?;
            validate::port(self.port as u32).map_err(|e| ValidationError { field: "port", err: e })?;
            validate::username(&self.username).map_err(|e| ValidationError { field: "username", err: e })?;
            validate::private_key_path(&self.private_key_path).map_err(|e| ValidationError { field: "private_key_path", err: e })?;
        }
        validate::shortcut(&self.shortcut).map_err(|e| ValidationError { field: "shortcut", err: e })?;
        Ok(())
    }

    /// Non-fatal warnings to show alongside a successful save. v1 only emits a loose-
    /// private-key-permissions warning on Unix hosts per spec's macOS note.
    pub fn warnings(&self) -> Vec<validate::FieldWarning> {
        let mut out = vec![];
        if self.mode == UploadMode::Ssh {
            if let Some(w) = validate::private_key_permissions(&self.private_key_path) {
                out.push(w);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn mode_defaults_to_local_when_field_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        ).unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.mode, UploadMode::Local);
    }

    #[test]
    fn auto_cleanup_defaults_to_false_when_field_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        ).unwrap();
        let cfg = load(&path).unwrap();
        assert!(!cfg.auto_cleanup);
    }

    #[test]
    fn local_mode_validate_skips_ssh_fields() {
        let mut cfg = Config::default();
        cfg.mode = UploadMode::Local;
        // host/username/etc. are all empty — should not fail in local mode
        assert!(cfg.validate().is_ok());
        // Verify the same config fails in SSH mode (confirms the guard is doing real work)
        let mut ssh_cfg = cfg.clone();
        ssh_cfg.mode = UploadMode::Ssh;
        assert!(ssh_cfg.validate().is_err());
    }

    #[test]
    fn ssh_mode_validate_still_requires_host() {
        let mut cfg = Config::default();
        cfg.mode = UploadMode::Ssh; // explicitly set SSH so host is validated
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.field, "host");
    }

    #[test]
    fn mode_and_auto_cleanup_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.mode = UploadMode::Local;
        cfg.auto_cleanup = true;
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.mode, UploadMode::Local);
        assert!(back.auto_cleanup);
    }

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

    #[test]
    fn load_rejects_large_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, r#"{"version":4294967297}"#).unwrap();
        let err = load(&path).unwrap_err();
        assert!(matches!(err, ConfigError::UnversionedOrUnsupported));
    }

    #[test]
    fn save_creates_nested_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("config").join("c.json");
        let cfg = Config::default();

        save(&path, &cfg).unwrap();

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), serde_json::to_string_pretty(&cfg).unwrap());
    }

    #[test]
    fn deserializes_without_shortcut_double_tap_field() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        )
        .unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.shortcut_double_tap, false);
    }

    #[test]
    fn round_trip_preserves_shortcut_double_tap() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.shortcut_double_tap = true;
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert!(back.shortcut_double_tap);
    }
}

#[cfg(test)]
mod validate_aggregate_tests {
    use super::*;

    #[test]
    fn ssh_mode_config_fails_validation_when_host_empty() {
        let mut cfg = Config::default();
        cfg.mode = UploadMode::Ssh;
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.field, "host");
    }

    #[test]
    fn local_mode_default_config_passes_validation() {
        let cfg = Config::default(); // mode = Local, shortcut pre-filled
        assert!(cfg.validate().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn warnings_surface_loose_key_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let f = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut cfg = Config::default();
        cfg.host = "example.com".into();
        cfg.username = "alice".into();
        cfg.remote_dir = "/uploads".into();
        cfg.private_key_path = f.path().to_string_lossy().into();
        assert!(!cfg.warnings().is_empty());
    }
}
