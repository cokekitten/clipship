use crate::config::Config;
use crate::ssh::{commands, runner::CommandRunner};
use rand::distributions::{Alphanumeric, DistString};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum TestConnectionError {
    #[error("config invalid: {0}")]
    ConfigInvalid(String),
    #[error("mkdir failed: {0}")]
    Mkdir(String),
    #[error("probe touch failed: {0}")]
    ProbeTouch(String),
    #[error("probe remove failed: {0}")]
    ProbeRemove(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn run(
    runner: Arc<dyn CommandRunner>,
    cfg: &Config,
) -> Result<(), TestConnectionError> {
    cfg.validate().map_err(|e| TestConnectionError::ConfigInvalid(e.to_string()))?;

    let probe_name = format!(
        ".clipship-probe-{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), 8).to_lowercase()
    );

    let mkdir_argv = commands::mkdir(
        cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &cfg.remote_dir,
    );
    let out = runner.run(mkdir_argv).await?;
    if !out.success {
        return Err(TestConnectionError::Mkdir(out.stderr));
    }

    let touch_argv = commands::probe_touch(
        cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &cfg.remote_dir, &probe_name,
    );
    let out = runner.run(touch_argv).await?;
    if !out.success {
        return Err(TestConnectionError::ProbeTouch(out.stderr));
    }

    let rm_argv = commands::probe_remove(
        cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &cfg.remote_dir, &probe_name,
    );
    let out = runner.run(rm_argv).await?;
    if !out.success {
        return Err(TestConnectionError::ProbeRemove(out.stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::runner::fakes::{fail_outcome, ok_outcome, RecordingRunner};

    fn cfg() -> Config {
        let f = tempfile::NamedTempFile::new().unwrap();
        let mut c = Config::default();
        c.host = "example.com".into();
        c.username = "alice".into();
        c.private_key_path = f.path().to_string_lossy().into();
        c.remote_dir = "/uploads".into();
        // Leak the tempfile so the path stays valid.
        std::mem::forget(f);
        c
    }

    #[tokio::test]
    async fn success_runs_mkdir_touch_remove_in_order() {
        let r = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()), Ok(ok_outcome()), Ok(ok_outcome()),
        ]);
        let c = cfg();
        run(std::sync::Arc::new(r.clone()), &c).await.unwrap();
        let calls = r.calls();
        assert_eq!(calls.len(), 3);
        assert!(calls[0].last().unwrap().starts_with("mkdir -p"));
        assert!(calls[1].last().unwrap().starts_with("touch "));
        assert!(calls[2].last().unwrap().starts_with("rm "));
    }

    #[tokio::test]
    async fn mkdir_failure_is_reported_and_probe_is_skipped() {
        let r = RecordingRunner::with_scripts(vec![Ok(fail_outcome(1, "denied"))]);
        let c = cfg();
        let err = run(std::sync::Arc::new(r.clone()), &c).await.unwrap_err();
        assert!(matches!(err, TestConnectionError::Mkdir(_)));
        assert_eq!(r.calls().len(), 1);
    }

    #[tokio::test]
    async fn touch_failure_is_surfaced() {
        let r = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()), Ok(fail_outcome(1, "read only")),
        ]);
        let c = cfg();
        let err = run(std::sync::Arc::new(r), &c).await.unwrap_err();
        assert!(matches!(err, TestConnectionError::ProbeTouch(_)));
    }
}
