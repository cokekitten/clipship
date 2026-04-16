#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    UploadSucceeded(String),        // remote path
    UploadSucceededButClipboardChanged(String),
    ClipboardEmpty,
    ClipboardDirectoryUnsupported,
    ConfigInvalid(String),          // detail
    SshBinariesMissing,
    MkdirFailed(String),            // stderr
    UploadFailed { stderr: String, part_path: String },
    FinalPathAlreadyExists(String), // .part kept at this path
    UploadInProgress,
    ClipboardWriteFailed,
    LocalPathInvalid(String),
    IoFailed(String),
    ShortcutRegistrationFailed(String),
    QuitDuringUpload,
}

pub trait Notifier: Send + Sync {
    fn notify(&self, m: Message);
}

pub struct RealNotifier<R: tauri::Runtime> {
    pub app: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> Notifier for RealNotifier<R> {
    fn notify(&self, m: Message) {
        use tauri_plugin_notification::NotificationExt;
        let (title, body) = render(&m);
        let _ = self.app.notification().builder().title(title).body(body).show();
    }
}

pub fn render(m: &Message) -> (&'static str, String) {
    match m {
        Message::UploadSucceeded(p) => ("Clipship", format!("Uploaded. Remote path copied: {p}")),
        Message::UploadSucceededButClipboardChanged(p) => (
            "Clipship",
            format!("Uploaded. Clipboard changed, path not copied. {p}"),
        ),
        Message::ClipboardEmpty => ("Clipship", "Clipboard has no uploadable file or image.".into()),
        Message::ClipboardDirectoryUnsupported => {
            ("Clipship", "Clipboard contains a directory, which is not supported.".into())
        }
        Message::ConfigInvalid(d) => ("Clipship", format!("Configuration is invalid: {d}")),
        Message::SshBinariesMissing => ("Clipship", "ssh or scp is missing on this machine.".into()),
        Message::MkdirFailed(e) => ("Clipship", format!("Remote directory creation failed: {e}")),
        Message::UploadFailed { stderr, part_path } => (
            "Clipship",
            format!("Upload failed: {stderr}. Remote partial file may be at: {part_path}"),
        ),
        Message::FinalPathAlreadyExists(p) => (
            "Clipship",
            format!("Upload aborted: destination already exists. Partial kept at {p}"),
        ),
        Message::UploadInProgress => ("Clipship", "Upload already in progress.".into()),
        Message::ClipboardWriteFailed => ("Clipship", "Upload succeeded but writing to clipboard failed.".into()),
        Message::LocalPathInvalid(p) => (
            "Clipship",
            format!("Local file path is not valid UTF-8 and cannot be passed to scp: {p}"),
        ),
        Message::IoFailed(e) => ("Clipship", format!("Local operation failed: {e}")),
        Message::ShortcutRegistrationFailed(e) => (
            "Clipship",
            format!("Global shortcut could not be registered: {e}. Check system shortcut permissions and conflicts."),
        ),
        Message::QuitDuringUpload => {
            ("Clipship", "Quit while uploading. Remote .part file may remain.".into())
        }
    }
}

#[cfg(test)]
pub mod fakes {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct RecordingNotifier {
        pub msgs: Arc<Mutex<Vec<Message>>>,
    }

    impl RecordingNotifier {
        pub fn msgs(&self) -> Vec<Message> {
            self.msgs.lock().unwrap().clone()
        }
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, m: Message) {
            self.msgs.lock().unwrap().push(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_covers_all_variants() {
        for m in [
            Message::UploadSucceeded("/x".into()),
            Message::UploadSucceededButClipboardChanged("/x".into()),
            Message::ClipboardEmpty,
            Message::ClipboardDirectoryUnsupported,
            Message::ConfigInvalid("x".into()),
            Message::SshBinariesMissing,
            Message::MkdirFailed("x".into()),
            Message::UploadFailed { stderr: "x".into(), part_path: "/x".into() },
            Message::FinalPathAlreadyExists("/x".into()),
            Message::UploadInProgress,
            Message::ClipboardWriteFailed,
            Message::LocalPathInvalid("/x".into()),
            Message::IoFailed("x".into()),
            Message::ShortcutRegistrationFailed("x".into()),
            Message::QuitDuringUpload,
        ] {
            let (_t, b) = render(&m);
            assert!(!b.is_empty());
        }
    }
}
