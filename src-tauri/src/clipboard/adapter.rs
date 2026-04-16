use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardContent {
    Files(Vec<PathBuf>),
    Image(Vec<u8>),
    Other,
    Empty,
}

pub trait ClipboardAdapter: Send + Sync {
    fn read(&self) -> ClipboardContent;
    fn write_text(&self, text: &str) -> Result<(), String>;
}

pub struct RealClipboard;

impl ClipboardAdapter for RealClipboard {
    fn read(&self) -> ClipboardContent {
        use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};
        let ctx = match ClipboardContext::new() {
            Ok(c) => c,
            Err(_) => return ClipboardContent::Empty,
        };
        if ctx.has(ContentFormat::Files) {
            if let Ok(files) = ctx.get_files() {
                let paths = files.into_iter().map(PathBuf::from).collect::<Vec<_>>();
                if !paths.is_empty() {
                    return ClipboardContent::Files(paths);
                }
            }
        }
        if ctx.has(ContentFormat::Image) {
            if let Ok(img) = ctx.get_image() {
                use clipboard_rs::common::RustImage;
                if let Ok(png) = img.to_png() {
                    return ClipboardContent::Image(png.get_bytes().to_vec());
                }
            }
        }
        if ctx.has(ContentFormat::Text) {
            return ClipboardContent::Other;
        }
        ClipboardContent::Empty
    }

    fn write_text(&self, text: &str) -> Result<(), String> {
        use clipboard_rs::{Clipboard, ClipboardContext};
        let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
        ctx.set_text(text.to_string()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
pub mod fakes {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    pub struct FakeClipboard {
        pub content: Arc<Mutex<ClipboardContent>>,
        pub written: Arc<Mutex<Vec<String>>>,
        pub write_should_fail: Arc<Mutex<bool>>,
    }

    impl Default for FakeClipboard {
        fn default() -> Self {
            Self::new(ClipboardContent::Empty)
        }
    }

    impl FakeClipboard {
        pub fn new(content: ClipboardContent) -> Self {
            Self {
                content: Arc::new(Mutex::new(content)),
                written: Arc::new(Mutex::new(vec![])),
                write_should_fail: Arc::new(Mutex::new(false)),
            }
        }

        pub fn set(&self, c: ClipboardContent) {
            *self.content.lock().unwrap() = c;
        }

        pub fn written(&self) -> Vec<String> {
            self.written.lock().unwrap().clone()
        }
    }

    impl ClipboardAdapter for FakeClipboard {
        fn read(&self) -> ClipboardContent {
            self.content.lock().unwrap().clone()
        }

        fn write_text(&self, text: &str) -> Result<(), String> {
            if *self.write_should_fail.lock().unwrap() {
                return Err("fake write failure".into());
            }
            self.written.lock().unwrap().push(text.to_string());
            *self.content.lock().unwrap() = ClipboardContent::Other;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fakes::FakeClipboard;
    use super::*;

    #[test]
    fn fake_round_trips_read_and_write() {
        let c = FakeClipboard::new(ClipboardContent::Files(vec!["/tmp/a".into()]));
        assert!(matches!(c.read(), ClipboardContent::Files(_)));
        c.write_text("hello").unwrap();
        assert_eq!(c.written(), vec!["hello"]);
    }
}
