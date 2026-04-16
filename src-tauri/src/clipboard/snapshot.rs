use crate::clipboard::adapter::ClipboardContent;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snapshot {
    Files(Vec<PathBuf>),
    ImageSha256([u8; 32]),
    Other,
    Empty,
}

impl Snapshot {
    pub fn of(content: &ClipboardContent) -> Snapshot {
        match content {
            ClipboardContent::Files(p) => Snapshot::Files(p.clone()),
            ClipboardContent::Image(bytes) => {
                let mut h = Sha256::new();
                h.update(bytes);
                let out: [u8; 32] = h.finalize().into();
                Snapshot::ImageSha256(out)
            }
            ClipboardContent::Other => Snapshot::Other,
            ClipboardContent::Empty => Snapshot::Empty,
        }
    }

    /// True iff the current clipboard content is indistinguishable from this snapshot
    /// by the app's comparison rules. Any kind transition is "changed".
    pub fn matches(&self, current: &ClipboardContent) -> bool {
        match (self, current) {
            (Snapshot::Files(a), ClipboardContent::Files(b)) => a == b,
            (Snapshot::ImageSha256(hash), ClipboardContent::Image(bytes)) => {
                let mut h = Sha256::new();
                h.update(bytes);
                let got: [u8; 32] = h.finalize().into();
                &got == hash
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_match_when_list_identical() {
        let c = ClipboardContent::Files(vec!["/a".into(), "/b".into()]);
        let snap = Snapshot::of(&c);
        assert!(snap.matches(&c));
    }

    #[test]
    fn files_mismatch_when_order_differs() {
        let c1 = ClipboardContent::Files(vec!["/a".into(), "/b".into()]);
        let c2 = ClipboardContent::Files(vec!["/b".into(), "/a".into()]);
        assert!(!Snapshot::of(&c1).matches(&c2));
    }

    #[test]
    fn image_matches_by_sha() {
        let c = ClipboardContent::Image(vec![1, 2, 3, 4]);
        let snap = Snapshot::of(&c);
        assert!(snap.matches(&c));
    }

    #[test]
    fn image_mismatch_when_bytes_differ() {
        let snap = Snapshot::of(&ClipboardContent::Image(vec![1, 2, 3]));
        assert!(!snap.matches(&ClipboardContent::Image(vec![1, 2, 3, 4])));
    }

    #[test]
    fn kind_transition_counts_as_changed() {
        let snap = Snapshot::of(&ClipboardContent::Files(vec!["/a".into()]));
        assert!(!snap.matches(&ClipboardContent::Image(vec![1])));
        assert!(!snap.matches(&ClipboardContent::Other));
        assert!(!snap.matches(&ClipboardContent::Empty));
    }

    #[test]
    fn other_and_empty_never_match() {
        let s = Snapshot::of(&ClipboardContent::Other);
        assert!(!s.matches(&ClipboardContent::Other));
    }
}
