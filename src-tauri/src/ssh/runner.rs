use tokio::process::Command;

/// Result of running a subprocess.  Cross-platform: we only need a success bit, stdout,
/// and stderr — we deliberately do not expose the raw ExitStatus so tests can construct
/// fake outcomes on any OS.
#[derive(Debug, Clone)]
pub struct CmdOutcome {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait::async_trait]
pub trait CommandRunner: Send + Sync {
    async fn run(&self, argv: Vec<String>) -> std::io::Result<CmdOutcome>;
}

pub struct TokioRunner;

#[async_trait::async_trait]
impl CommandRunner for TokioRunner {
    async fn run(&self, argv: Vec<String>) -> std::io::Result<CmdOutcome> {
        let mut it = argv.into_iter();
        let program = it.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty argv")
        })?;

        #[allow(unused_mut)]
        let mut cmd = Command::new(program);
        cmd.args(it);

        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let out = cmd.output().await?;
        Ok(CmdOutcome {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }
}

#[cfg(test)]
pub mod fakes {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct RecordingRunner {
        pub calls: Arc<Mutex<Vec<Vec<String>>>>,
        pub script: Arc<Mutex<Vec<Result<CmdOutcome, std::io::Error>>>>,
    }

    impl RecordingRunner {
        pub fn with_scripts(scripts: Vec<Result<CmdOutcome, std::io::Error>>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                script: Arc::new(Mutex::new(scripts)),
            }
        }

        pub fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl CommandRunner for RecordingRunner {
        async fn run(&self, argv: Vec<String>) -> std::io::Result<CmdOutcome> {
            self.calls.lock().unwrap().push(argv);
            let mut s = self.script.lock().unwrap();
            if s.is_empty() {
                Ok(ok_outcome())
            } else {
                s.remove(0)
            }
        }
    }

    pub fn ok_outcome() -> CmdOutcome {
        CmdOutcome { success: true, stdout: String::new(), stderr: String::new() }
    }

    pub fn fail_outcome(_code: i32, stderr: &str) -> CmdOutcome {
        CmdOutcome { success: false, stdout: String::new(), stderr: stderr.to_string() }
    }
}
