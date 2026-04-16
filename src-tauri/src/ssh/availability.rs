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
    #[allow(unused_mut)]
    let mut ssh_cmd = Command::new(SSH_BIN);
    ssh_cmd.arg("-V");
    #[allow(unused_mut)]
    let mut scp_cmd = Command::new(SCP_BIN);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        ssh_cmd.creation_flags(CREATE_NO_WINDOW);
        scp_cmd.creation_flags(CREATE_NO_WINDOW);
    }

    Availability {
        ssh: ssh_cmd.output().await.is_ok(),
        scp: scp_cmd.output().await.is_ok(),
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
