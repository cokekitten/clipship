use tokio::process::Command;

#[cfg(windows)]
pub const SSH_BIN: &str = "ssh.exe";
#[cfg(windows)]
pub const SCP_BIN: &str = "scp.exe";
#[cfg(not(windows))]
pub const SSH_BIN: &str = "ssh";
#[cfg(not(windows))]
pub const SCP_BIN: &str = "scp";

#[derive(Debug, Clone, Default)]
pub struct Availability {
    pub ssh: bool,
    pub scp: bool,
}

/// Detect whether ssh and scp binaries are reachable.  "Reachable" means we could spawn
/// the process — the actual exit status does not matter because scp has no -V flag and
/// will return non-zero when invoked without arguments, which is still proof of existence.
pub async fn check() -> Availability {
    Availability {
        ssh: Command::new(SSH_BIN).arg("-V").output().await.is_ok(),
        scp: Command::new(SCP_BIN).output().await.is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn check_returns_without_panicking() {
        let a = check().await;
        // On a developer workstation ssh and scp are usually present; in CI they may not be.
        // Assert only that the call completes.
        let _ = (a.ssh, a.scp);
    }
}
