use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

use tauri_plugin_global_shortcut::Shortcut;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FieldError {
    #[error("host is empty")]
    HostEmpty,
    #[error("host has invalid characters or starts with '-'")]
    HostInvalid,
    #[error("username is empty")]
    UsernameEmpty,
    #[error("username has invalid characters or starts with '-'")]
    UsernameInvalid,
    #[error("port must be between 1 and 65535")]
    PortOutOfRange,
    #[error("remote_dir must start with '/'")]
    RemoteDirNotAbsolute,
    #[error("remote_dir contains '..' path segment")]
    RemoteDirTraversal,
    #[error("remote_dir contains unsafe characters")]
    RemoteDirUnsafe,
    #[error("remote_dir contains non-ASCII characters; v1 does not support them")]
    RemoteDirNonAscii,
    #[error("private_key_path does not exist")]
    KeyMissing,
    #[error("shortcut is empty")]
    ShortcutEmpty,
    #[error("shortcut has invalid syntax")]
    ShortcutInvalid,
}

/// Non-fatal warnings the UI surfaces alongside a successful validate().
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldWarning {
    /// On macOS/Linux, OpenSSH rejects keys whose group/other bits allow read.
    PrivateKeyLoosePermissions { path: String, mode: u32 },
}

pub fn host(s: &str) -> Result<(), FieldError> {
    if s.is_empty() {
        return Err(FieldError::HostEmpty);
    }
    if s.starts_with('-') {
        return Err(FieldError::HostInvalid);
    }
    if s.contains(':') {
        match IpAddr::from_str(s) {
            Ok(IpAddr::V6(_)) => Ok(()),
            _ => Err(FieldError::HostInvalid),
        }
    } else if s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        Ok(())
    } else {
        Err(FieldError::HostInvalid)
    }
}

pub fn username(s: &str) -> Result<(), FieldError> {
    if s.is_empty() {
        return Err(FieldError::UsernameEmpty);
    }
    if s.starts_with('-') {
        return Err(FieldError::UsernameInvalid);
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(FieldError::UsernameInvalid);
    }
    Ok(())
}

pub fn port(p: u32) -> Result<(), FieldError> {
    if !(1..=65535).contains(&p) {
        return Err(FieldError::PortOutOfRange);
    }
    Ok(())
}

pub fn remote_dir(s: &str) -> Result<(), FieldError> {
    if !s.is_ascii() {
        return Err(FieldError::RemoteDirNonAscii);
    }
    if !s.starts_with('/') {
        return Err(FieldError::RemoteDirNotAbsolute);
    }
    for segment in s.split('/') {
        if segment == ".." {
            return Err(FieldError::RemoteDirTraversal);
        }
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
    {
        return Err(FieldError::RemoteDirUnsafe);
    }
    Ok(())
}

pub fn private_key_path(p: &str) -> Result<(), FieldError> {
    if !Path::new(p).is_file() {
        return Err(FieldError::KeyMissing);
    }
    Ok(())
}

/// Returns a warning if the key file permissions look loose enough that OpenSSH will reject it.
/// On Windows this is a no-op because Unix mode bits don't apply.
#[cfg(unix)]
pub fn private_key_permissions(p: &str) -> Option<FieldWarning> {
    use std::os::unix::fs::PermissionsExt;
    let meta = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return None,
    };
    let mode = meta.permissions().mode() & 0o777;
    // group or other bits set -> too permissive for OpenSSH.
    if mode & 0o077 != 0 {
        Some(FieldWarning::PrivateKeyLoosePermissions {
            path: p.to_string(),
            mode,
        })
    } else {
        None
    }
}

#[cfg(not(unix))]
pub fn private_key_permissions(_p: &str) -> Option<FieldWarning> {
    None
}

pub fn shortcut(s: &str) -> Result<(), FieldError> {
    if s.trim().is_empty() {
        return Err(FieldError::ShortcutEmpty);
    }
    if Shortcut::from_str(s).is_err() {
        return Err(FieldError::ShortcutInvalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_accepts_common_forms() {
        // Config accepts bare IPv6 literals. Brackets are added only when constructing
        // the scp target; users should enter `::1`, not `[::1]`.
        for ok in ["example.com", "10.0.0.1", "::1", "my_host"] {
            host(ok).unwrap_or_else(|e| panic!("{ok}: {e:?}"));
        }
    }

    #[test]
    fn host_rejects_arg_injection_and_space() {
        assert!(host("-oProxyCommand=evil").is_err());
        assert!(host("example com").is_err());
        assert!(host("[::1]").is_err());
        assert!(host("").is_err());
    }

    #[test]
    fn host_rejects_malformed_colon_strings() {
        assert!(host("example:foo").is_err());
        assert!(host("host:22").is_err());
        assert!(host("not::quite::right").is_err());
    }

    #[test]
    fn username_rules() {
        assert!(username("alice").is_ok());
        assert!(username("alice_01").is_ok());
        assert!(username("").is_err());
        assert!(username("-evil").is_err());
        assert!(username("has space").is_err());
        assert!(username("has$dollar").is_err());
    }

    #[test]
    fn port_range() {
        assert!(port(22).is_ok());
        assert!(port(1).is_ok());
        assert!(port(65535).is_ok());
        assert!(port(0).is_err());
        assert!(port(65536).is_err());
    }

    #[test]
    fn remote_dir_rules() {
        assert!(remote_dir("/home/ubuntu/uploads").is_ok());
        assert!(remote_dir("home/ubuntu").is_err()); // not absolute
        assert!(remote_dir("/home/../etc").is_err()); // traversal
        assert!(remote_dir("/home/user with space").is_err()); // space
        assert!(remote_dir("/home/`whoami`").is_err()); // backtick
        assert!(remote_dir("/home/$(id)").is_err()); // dollar
        assert!(remote_dir("/home/上传").is_err()); // non-ASCII
        assert!(remote_dir("/home/\nuser").is_err()); // newline
    }

    #[test]
    fn shortcut_rules() {
        assert!(shortcut("CmdOrCtrl+Shift+U").is_ok());
        assert!(matches!(shortcut("not a shortcut"), Err(FieldError::ShortcutInvalid)));
        assert!(matches!(shortcut("CmdOrCtrl++"), Err(FieldError::ShortcutInvalid)));
        assert!(shortcut("   ").is_err());
    }

    #[test]
    fn private_key_path_checks_existence() {
        assert!(private_key_path("/definitely/does/not/exist/xyz").is_err());
        let f = tempfile::NamedTempFile::new().unwrap();
        assert!(private_key_path(f.path().to_str().unwrap()).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn private_key_permissions_warns_on_group_or_other_readable() {
        use std::os::unix::fs::PermissionsExt;
        let f = tempfile::NamedTempFile::new().unwrap();
        let p = f.path().to_str().unwrap();

        // Tight: 0o600 -> no warning.
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(private_key_permissions(p).is_none());

        // Loose: 0o644 -> warn.
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        match private_key_permissions(p) {
            Some(FieldWarning::PrivateKeyLoosePermissions { mode, .. }) => {
                assert_eq!(mode & 0o077, 0o044);
            }
            other => panic!("expected loose-perm warning, got {other:?}"),
        }
    }
}
