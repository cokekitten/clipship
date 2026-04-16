use chrono::Utc;
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;

/// Returns `(stem, ext)` where stem is the sanitized base (without extension) and ext is a
/// sanitized extension including the dot, or empty string if none.
pub fn sanitize(original: &str) -> (String, String) {
    let (stem_raw, ext_raw) = split_ext(original);
    let stem = sanitize_segment(&stem_raw);
    let stem = if stem.is_empty() { "file".to_string() } else { stem };
    let ext = if ext_raw.is_empty() {
        String::new()
    } else {
        format!(".{}", sanitize_segment(&ext_raw))
    };
    (stem, ext)
}

fn split_ext(s: &str) -> (String, String) {
    match s.rsplit_once('.') {
        Some((a, b)) if !a.is_empty() && !b.is_empty() => (a.to_string(), b.to_string()),
        _ => (s.to_string(), String::new()),
    }
}

fn sanitize_segment(s: &str) -> String {
    // Collapse whitespace runs to '-'.
    let ws = Regex::new(r"\s+").unwrap();
    let s = ws.replace_all(s, "-").to_string();
    // Drop any character outside the safe set.
    let safe = Regex::new(r"[^A-Za-z0-9._\-]").unwrap();
    safe.replace_all(&s, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_simple_name() {
        assert_eq!(sanitize("report.pdf"), ("report".into(), ".pdf".into()));
    }

    #[test]
    fn replaces_whitespace_and_drops_unsafe() {
        assert_eq!(sanitize("my cool file.png"), ("my-cool-file".into(), ".png".into()));
        assert_eq!(sanitize("weird;/name.txt"), ("weirdname".into(), ".txt".into()));
    }

    #[test]
    fn drops_control_and_separator_chars() {
        let input = "bad\nname/with\\stuff.tar.gz";
        let (stem, ext) = sanitize(input);
        assert!(!stem.contains('\n'));
        assert!(!stem.contains('/'));
        assert!(!stem.contains('\\'));
        assert_eq!(ext, ".gz");
    }

    #[test]
    fn empty_stem_becomes_file() {
        assert_eq!(sanitize("///").0, "file");
        assert_eq!(sanitize("").0, "file");
    }

    #[test]
    fn non_ascii_is_dropped() {
        assert_eq!(sanitize("上传.pdf"), ("file".into(), ".pdf".into()));
    }

    #[test]
    fn no_extension_yields_empty_ext() {
        assert_eq!(sanitize("README"), ("README".into(), "".into()));
    }
}

/// Build the final remote filename `YYYYMMDD-HHMMSS-SSS-<rand6>-<stem>.<ext>`.
pub fn build_remote_filename(original: &str) -> String {
    let (stem, ext) = sanitize(original);
    let now = Utc::now();
    let ts = now.format("%Y%m%d-%H%M%S").to_string();
    let ms = now.timestamp_subsec_millis();
    let rand: String = Alphanumeric
        .sample_string(&mut rand::thread_rng(), 6)
        .to_lowercase();
    format!("{ts}-{ms:03}-{rand}-{stem}{ext}")
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn format_matches_spec() {
        let name = build_remote_filename("report.pdf");
        let re =
            Regex::new(r"^\d{8}-\d{6}-\d{3}-[a-z0-9]{6}-report\.pdf$").unwrap();
        assert!(re.is_match(&name), "unexpected: {name}");
    }

    #[test]
    fn clipboard_png_placeholder() {
        let name = build_remote_filename("clipboard.png");
        assert!(name.ends_with("-clipboard.png"));
    }

    #[test]
    fn two_rapid_calls_differ() {
        let a = build_remote_filename("x.bin");
        let b = build_remote_filename("x.bin");
        // The rand6 portion must prevent collisions even within the same millisecond.
        assert_ne!(a, b);
    }
}
