use crate::config::Config;
use crate::ssh::runner::CommandRunner;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub fn is_ssh_complete(cfg: &Config) -> bool {
    !cfg.host.is_empty()
        && !cfg.username.is_empty()
        && !cfg.private_key_path.is_empty()
        && !cfg.remote_dir.is_empty()
}

/// Delete files in `dir` older than `max_age`. Silently skips unreadable entries.
pub fn cleanup_local(dir: &Path, max_age: Duration) {
    if !dir.exists() {
        return;
    }
    let threshold = match SystemTime::now().checked_sub(max_age) {
        Some(t) => t,
        None => return,
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cleanup_local read_dir {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let modified = match meta.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if modified < threshold {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                eprintln!("cleanup_local remove {}: {e}", entry.path().display());
            }
        }
    }
}

/// Run `find <remote_dir> -maxdepth 1 -mtime +7 -type f -delete` via SSH.
/// No-op when SSH config is incomplete. Errors are logged, not propagated.
pub async fn cleanup_remote(cfg: &Config, runner: &dyn CommandRunner) {
    if !is_ssh_complete(cfg) {
        return;
    }
    let argv = crate::ssh::commands::find_and_delete_old(
        cfg.port,
        &cfg.private_key_path,
        &cfg.username,
        &cfg.host,
        &cfg.remote_dir,
    );
    match runner.run(argv).await {
        Ok(out) if !out.success => {
            eprintln!("cleanup_remote: remote find/delete failed: {}", out.stderr);
        }
        Err(e) => {
            eprintln!("cleanup_remote: ssh error: {e}");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn is_ssh_complete_false_when_any_field_empty() {
        let cfg = Config::default();
        assert!(!is_ssh_complete(&cfg));
    }

    #[test]
    fn is_ssh_complete_false_when_only_host_set() {
        let mut cfg = Config::default();
        cfg.host = "h".into();
        // username, private_key_path, remote_dir still empty
        assert!(!is_ssh_complete(&cfg));
    }

    #[test]
    fn is_ssh_complete_true_when_all_fields_set() {
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.username = "u".into();
        cfg.private_key_path = "/k".into();
        cfg.remote_dir = "/tmp/clipship".into();
        assert!(is_ssh_complete(&cfg));
    }

    #[test]
    fn cleanup_local_noop_when_dir_absent() {
        let dir = tempdir().unwrap();
        let nonexistent = dir.path().join("nope");
        cleanup_local(&nonexistent, Duration::from_secs(1));
    }

    #[test]
    fn cleanup_local_deletes_files_older_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("old.txt");
        std::fs::write(&file, b"x").unwrap();
        // Sleep longer than max_age so the file's mtime is definitely older.
        // Using 10ms sleep + 1ms max_age avoids sub-millisecond FS timestamp races.
        std::thread::sleep(std::time::Duration::from_millis(10));
        cleanup_local(dir.path(), Duration::from_millis(1));
        assert!(!file.exists(), "file should have been deleted");
    }

    #[test]
    fn cleanup_local_keeps_files_newer_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new.txt");
        std::fs::write(&file, b"x").unwrap();
        cleanup_local(dir.path(), Duration::from_secs(3600));
        assert!(file.exists(), "file should have been kept");
    }
}
