use crate::naming::remote_path;
use crate::ssh::availability::{SCP_BIN, SSH_BIN};
use crate::ssh::opts::SSH_OPTIONS;

fn ssh_base(port: u16, key: &str, user: &str, host: &str) -> Vec<String> {
    let mut v = vec![SSH_BIN.to_string(), "-p".to_string(), port.to_string(), "-i".to_string(), key.to_string()];
    v.extend(SSH_OPTIONS.iter().map(|s| s.to_string()));
    v.push(remote_path::ssh_user_host(user, host));
    v
}

fn scp_base(port: u16, key: &str) -> Vec<String> {
    let mut v = vec![SCP_BIN.to_string(), "-P".to_string(), port.to_string(), "-i".to_string(), key.to_string()];
    v.extend(SSH_OPTIONS.iter().map(|s| s.to_string()));
    v
}

pub fn mkdir(port: u16, key: &str, user: &str, host: &str, remote_dir: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!("mkdir -p '{}'", remote_dir));
    v
}

pub fn rm_part(port: u16, key: &str, user: &str, host: &str, remote_part_path: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!("rm -f '{}'", remote_part_path));
    v
}

pub fn mv_no_overwrite(
    port: u16, key: &str, user: &str, host: &str,
    remote_part_path: &str, remote_final_path: &str,
) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!("mv -n '{}' '{}'", remote_part_path, remote_final_path));
    v
}

pub fn scp_upload(
    port: u16, key: &str, user: &str, host: &str,
    local_path: &str, remote_part_path: &str,
) -> Vec<String> {
    let mut v = scp_base(port, key);
    v.push(local_path.to_string());
    v.push(remote_path::scp_target(user, host, remote_part_path));
    v
}

pub fn probe_touch(port: u16, key: &str, user: &str, host: &str, remote_dir: &str, probe_name: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!("touch '{}/{}'", remote_dir, probe_name));
    v
}

pub fn probe_remove(port: u16, key: &str, user: &str, host: &str, remote_dir: &str, probe_name: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!("rm '{}/{}'", remote_dir, probe_name));
    v
}

pub fn find_and_delete_old(port: u16, key: &str, user: &str, host: &str, remote_dir: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!(
        "find '{}' -maxdepth 1 -mtime +7 -type f -delete",
        remote_dir
    ));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::availability::{SCP_BIN, SSH_BIN};

    #[test]
    fn mkdir_shape() {
        let v = mkdir(22, "/k", "u", "h.com", "/r");
        assert_eq!(v[0], SSH_BIN);
        assert_eq!(v[1], "-p");
        assert_eq!(v[2], "22");
        assert_eq!(v[3], "-i");
        assert_eq!(v[4], "/k");
        assert_eq!(v[v.len() - 2], "u@h.com");
        assert_eq!(v[v.len() - 1], "mkdir -p '/r'");
    }

    #[test]
    fn scp_preserves_windows_local_path() {
        let v = scp_upload(22, "/k", "u", "h.com", r"C:\Users\Me\My Docs\a.txt", "/r/a.part");
        assert!(v.iter().any(|s| s == r"C:\Users\Me\My Docs\a.txt"));
    }

    #[test]
    fn scp_uses_bracket_for_ipv6_target_but_not_ssh_userhost() {
        let v = scp_upload(22, "/k", "u", "::1", "/tmp/x", "/r/x.part");
        assert_eq!(v[0], SCP_BIN);
        assert_eq!(v[v.len() - 1], "u@[::1]:/r/x.part");
    }

    #[test]
    fn mv_uses_n_flag() {
        let v = mv_no_overwrite(22, "/k", "u", "h.com", "/r/x.part", "/r/x");
        assert!(v.last().unwrap().contains("mv -n"));
        assert!(v.last().unwrap().contains("/r/x.part"));
        assert!(v.last().unwrap().contains("/r/x"));
    }

    #[test]
    fn shared_options_appear_on_ssh_and_scp() {
        let a = mkdir(22, "/k", "u", "h.com", "/r");
        let b = scp_upload(22, "/k", "u", "h.com", "/tmp/x", "/r/x.part");
        for opt in ["BatchMode=yes", "StrictHostKeyChecking=accept-new", "ConnectTimeout=10"] {
            assert!(a.iter().any(|s| s == opt), "ssh missing {opt}");
            assert!(b.iter().any(|s| s == opt), "scp missing {opt}");
        }
    }

    #[test]
    fn probe_touch_and_remove_include_dir_and_name() {
        let t = probe_touch(22, "/k", "u", "h.com", "/r", ".clipship-probe-abc");
        let r = probe_remove(22, "/k", "u", "h.com", "/r", ".clipship-probe-abc");
        assert!(t.last().unwrap().contains("touch '/r/.clipship-probe-abc'"));
        assert!(r.last().unwrap().contains("rm '/r/.clipship-probe-abc'"));
    }

    #[test]
    fn find_and_delete_old_shape() {
        let v = find_and_delete_old(22, "/k", "u", "h.com", "/r");
        assert_eq!(v[0], SSH_BIN);
        assert!(v.last().unwrap().contains("find '/r' -maxdepth 1 -mtime +7 -type f -delete"));
    }
}
