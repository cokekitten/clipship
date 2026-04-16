/// Fixed `-o key=value` options applied to both ssh and scp.
pub const SSH_OPTIONS: &[&str] = &[
    "-o", "BatchMode=yes",
    "-o", "StrictHostKeyChecking=accept-new",
    "-o", "ConnectTimeout=10",
    "-o", "ServerAliveInterval=15",
    "-o", "ServerAliveCountMax=3",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_batch_mode() {
        assert!(SSH_OPTIONS.windows(2).any(|w| w == ["-o", "BatchMode=yes"]));
    }

    #[test]
    fn contains_accept_new() {
        assert!(SSH_OPTIONS.windows(2).any(|w| w == ["-o", "StrictHostKeyChecking=accept-new"]));
    }
}
