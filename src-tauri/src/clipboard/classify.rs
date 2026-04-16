use crate::clipboard::adapter::ClipboardContent;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum Classified {
    FileToUpload(PathBuf),
    DirectoryUnsupported,
    ImageBytes(Vec<u8>),
    Nothing,
}

pub fn classify(content: ClipboardContent) -> Classified {
    match content {
        ClipboardContent::Files(list) => {
            if let Some(first) = list.into_iter().next() {
                if first.is_dir() {
                    Classified::DirectoryUnsupported
                } else {
                    Classified::FileToUpload(first)
                }
            } else {
                Classified::Nothing
            }
        }
        ClipboardContent::Image(bytes) => Classified::ImageBytes(bytes),
        ClipboardContent::Other | ClipboardContent::Empty => Classified::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_picks_first() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let content = ClipboardContent::Files(vec![tmp.path().to_path_buf(), "/x/y".into()]);
        let c = classify(content);
        assert_eq!(c, Classified::FileToUpload(tmp.path().to_path_buf()));
    }

    #[test]
    fn directory_is_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let c = classify(ClipboardContent::Files(vec![dir.path().to_path_buf()]));
        assert_eq!(c, Classified::DirectoryUnsupported);
    }

    #[test]
    fn image_bytes_are_carried_through() {
        let c = classify(ClipboardContent::Image(vec![9, 9, 9]));
        assert_eq!(c, Classified::ImageBytes(vec![9, 9, 9]));
    }

    #[test]
    fn other_and_empty_become_nothing() {
        assert_eq!(classify(ClipboardContent::Other), Classified::Nothing);
        assert_eq!(classify(ClipboardContent::Empty), Classified::Nothing);
    }
}
