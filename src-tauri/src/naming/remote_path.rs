/// Join a remote directory (absolute, validated) and a filename.
pub fn join(remote_dir: &str, filename: &str) -> String {
    if remote_dir.ends_with('/') {
        format!("{remote_dir}{filename}")
    } else {
        format!("{remote_dir}/{filename}")
    }
}

/// Append the `.part` suffix for the atomic-rename staging file.
pub fn part_path(final_path: &str) -> String {
    format!("{final_path}.part")
}

/// Build the `user@host` form for SSH.  Hostnames, IPv4 literals, and bare IPv6 literals
/// are passed unbracketed.
pub fn ssh_user_host(user: &str, host: &str) -> String {
    format!("{user}@{host}")
}

/// Build the `user@host:path` form for SCP.  IPv6 literal hosts MUST be bracketed here,
/// otherwise scp cannot distinguish the host from the remote path's colons.
pub fn scp_target(user: &str, host: &str, remote_path: &str) -> String {
    if is_ipv6_literal(host) {
        format!("{user}@[{host}]:{remote_path}")
    } else {
        format!("{user}@{host}:{remote_path}")
    }
}

/// An IPv6 literal is detected by presence of `:` without a surrounding `[...]`.
/// Hostnames never contain `:`, IPv4 literals never contain `:`, and pre-bracketed
/// input is treated as already bracketed and passed through.
fn is_ipv6_literal(host: &str) -> bool {
    host.contains(':') && !host.starts_with('[')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_handles_trailing_slash() {
        assert_eq!(join("/a/b", "c.txt"), "/a/b/c.txt");
        assert_eq!(join("/a/b/", "c.txt"), "/a/b/c.txt");
    }

    #[test]
    fn part_path_adds_suffix() {
        assert_eq!(part_path("/a/b/c.txt"), "/a/b/c.txt.part");
    }

    #[test]
    fn ssh_user_host_never_brackets() {
        assert_eq!(ssh_user_host("u", "example.com"), "u@example.com");
        assert_eq!(ssh_user_host("u", "::1"), "u@::1");
        assert_eq!(ssh_user_host("u", "10.0.0.1"), "u@10.0.0.1");
    }

    #[test]
    fn scp_target_brackets_only_ipv6() {
        assert_eq!(
            scp_target("u", "example.com", "/a/b.txt"),
            "u@example.com:/a/b.txt"
        );
        assert_eq!(
            scp_target("u", "10.0.0.1", "/a/b.txt"),
            "u@10.0.0.1:/a/b.txt"
        );
        assert_eq!(
            scp_target("u", "::1", "/a/b.txt"),
            "u@[::1]:/a/b.txt"
        );
        assert_eq!(
            scp_target("u", "fe80::1", "/a/b.txt"),
            "u@[fe80::1]:/a/b.txt"
        );
    }
}
