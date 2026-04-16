use crate::clipboard::adapter::{ClipboardAdapter, ClipboardContent};
use crate::clipboard::classify::{classify, Classified};
use crate::clipboard::image::TempImage;
use crate::clipboard::snapshot::Snapshot;
use crate::config::Config;
use crate::naming::{filename, remote_path};
use crate::notify::{Message, Notifier};
use crate::ssh::{commands, runner::CommandRunner};
use crate::upload::errors::UploadError;
use crate::upload::guard::InFlightGuard;
use std::path::PathBuf;
use std::sync::Arc;

pub struct UploadService {
    pub runner: Arc<dyn CommandRunner>,
    pub clipboard: Arc<dyn ClipboardAdapter>,
    pub notifier: Arc<dyn Notifier>,
    pub guard: InFlightGuard,
    pub temp_dir: PathBuf,
    pub local_output_dir: PathBuf,
    pub last_uploaded: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    #[cfg(test)]
    pub after_snapshot_hook: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}

#[derive(Debug)]
pub struct UploadSuccess {
    pub remote_path: String,
    pub clipboard_updated: bool,
}

impl UploadService {
    /// Public entry: run the upload, and ALWAYS emit exactly one user-facing notification.
    pub async fn upload(&self, cfg: &Config) -> Result<UploadSuccess, UploadError> {
        let result = self.upload_inner(cfg).await;
        match &result {
            Ok(s) => {
                self.notifier.notify(if s.clipboard_updated {
                    Message::UploadSucceeded(s.remote_path.clone())
                } else {
                    Message::UploadSucceededButClipboardChanged(s.remote_path.clone())
                });
            }
            Err(e) => {
                if let Some(msg) = error_to_message(e) {
                    self.notifier.notify(msg);
                }
            }
        }
        result
    }

    async fn upload_inner(&self, cfg: &Config) -> Result<UploadSuccess, UploadError> {
        cfg.validate()
            .map_err(|e| UploadError::ConfigInvalid(e.to_string()))?;

        let _token = self
            .guard
            .try_acquire()
            .ok_or(UploadError::InProgress)?;

        let content = self.clipboard.read();

        match cfg.mode {
            crate::config::UploadMode::Ssh => self.upload_via_ssh(cfg, content).await,
            crate::config::UploadMode::Local => self.upload_via_local(content).await,
        }
    }

    async fn upload_via_ssh(&self, cfg: &Config, content: ClipboardContent) -> Result<UploadSuccess, UploadError> {
        let (local_path, original_name, temp): (PathBuf, String, Option<TempImage>) =
            match classify(content.clone()) {
                Classified::FileToUpload(p) => {
                    let name = p
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("file")
                        .to_string();
                    (p, name, None)
                }
                Classified::ImageBytes(bytes) => {
                    let img = TempImage::write(&self.temp_dir, &bytes)?;
                    (img.path.clone(), "clipboard.png".into(), Some(img))
                }
                Classified::DirectoryUnsupported => return Err(UploadError::ClipboardDirectory),
                Classified::Nothing => return Err(UploadError::ClipboardEmpty),
            };

        // remote_dir is auto-detected and stored at save time; fall back for old configs.
        let remote_dir = if cfg.remote_dir.is_empty() { "/tmp/clipship" } else { &cfg.remote_dir };

        let remote_name = filename::build_remote_filename(&original_name);
        let remote_final = remote_path::join(remote_dir, &remote_name);
        let remote_part = remote_path::part_path(&remote_final);

        let snap = Snapshot::of(&content);

        #[cfg(test)]
        if let Some(h) = &self.after_snapshot_hook {
            h();
        }

        let mkdir_argv = commands::mkdir(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, remote_dir,
        );
        let out = self.runner.run(mkdir_argv).await?;
        if !out.success {
            return Err(UploadError::MkdirFailed(out.stderr));
        }

        let rm_argv = commands::rm_part(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &remote_part,
        );
        let _ = self.runner.run(rm_argv).await?; // rm -f must not fail the upload

        let local_path_text = local_path.to_str().ok_or_else(|| {
            UploadError::LocalPathInvalid(local_path.to_string_lossy().to_string())
        })?;

        let scp_argv = commands::scp_upload(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host,
            local_path_text,
            &remote_part,
        );
        let out = self.runner.run(scp_argv).await?;
        if !out.success {
            return Err(UploadError::ScpFailed {
                stderr: out.stderr,
                part_path: remote_part,
            });
        }

        let mv_argv = commands::mv_no_overwrite(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host,
            &remote_part, &remote_final,
        );
        let out = self.runner.run(mv_argv).await?;
        if !out.success {
            return Err(UploadError::FinalExists(remote_part));
        }

        let current = self.clipboard.read();
        let clipboard_updated = if snap.matches(&current) {
            self.clipboard
                .write_text(&remote_final)
                .map_err(|_| UploadError::ClipboardWrite)?;
            true
        } else {
            false
        };

        *self.last_uploaded.lock().unwrap() = Some(remote_final.clone());

        if let Some(img) = temp {
            img.delete();
        }

        Ok(UploadSuccess {
            remote_path: remote_final,
            clipboard_updated,
        })
    }

    async fn upload_via_local(&self, content: ClipboardContent) -> Result<UploadSuccess, UploadError> {
        let (src_path, original_name, bytes): (Option<PathBuf>, String, Option<Vec<u8>>) =
            match classify(content) {
                Classified::FileToUpload(p) => {
                    let name = p
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("file")
                        .to_string();
                    (Some(p), name, None)
                }
                Classified::ImageBytes(b) => (None, "clipboard.png".into(), Some(b)),
                Classified::DirectoryUnsupported => return Err(UploadError::ClipboardDirectory),
                Classified::Nothing => return Err(UploadError::ClipboardEmpty),
            };

        let dest_name = filename::build_remote_filename(&original_name);
        std::fs::create_dir_all(&self.local_output_dir)?;
        let dest = self.local_output_dir.join(&dest_name);

        if let Some(src) = src_path {
            std::fs::copy(&src, &dest)?;
        } else if let Some(b) = bytes {
            std::fs::write(&dest, &b)?;
        }

        let dest_str = dest
            .to_str()
            .ok_or_else(|| UploadError::LocalPathInvalid(dest.to_string_lossy().into()))?
            .to_string();

        self.clipboard
            .write_text(&dest_str)
            .map_err(|_| UploadError::ClipboardWrite)?;
        *self.last_uploaded.lock().unwrap() = Some(dest_str.clone());

        Ok(UploadSuccess { remote_path: dest_str, clipboard_updated: true })
    }
}

/// Map each UploadError variant onto the user-visible Message it should emit.  Returns
/// None only for variants whose notification is handled elsewhere (currently none, but
/// left open for the future `Io` sink-hole variant).
fn error_to_message(err: &UploadError) -> Option<Message> {
    Some(match err {
        UploadError::ConfigInvalid(s) => Message::ConfigInvalid(s.clone()),
        UploadError::BinariesMissing => Message::SshBinariesMissing,
        UploadError::ClipboardEmpty => Message::ClipboardEmpty,
        UploadError::ClipboardDirectory => Message::ClipboardDirectoryUnsupported,
        UploadError::MkdirFailed(s) => Message::MkdirFailed(s.clone()),
        UploadError::ScpFailed { stderr, part_path } => Message::UploadFailed {
            stderr: stderr.clone(),
            part_path: part_path.clone(),
        },
        UploadError::FinalExists(p) => Message::FinalPathAlreadyExists(p.clone()),
        UploadError::InProgress => Message::UploadInProgress,
        UploadError::ClipboardWrite => Message::ClipboardWriteFailed,
        UploadError::LocalPathInvalid(p) => Message::LocalPathInvalid(p.clone()),
        UploadError::Io(e) => Message::IoFailed(e.to_string()),
    })
}

#[cfg(test)]
mod happy_path_tests {
    use super::*;
    use crate::clipboard::adapter::{fakes::FakeClipboard, ClipboardContent};
    use crate::notify::fakes::RecordingNotifier;
    use crate::ssh::runner::fakes::{ok_outcome, RecordingRunner};

    fn valid_cfg(key_path: &str) -> Config {
        Config {
            version: 1,
            mode: crate::config::UploadMode::Ssh,
            host: "example.com".into(),
            port: 22,
            username: "alice".into(),
            private_key_path: key_path.into(),
            remote_dir: "/uploads".into(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
            auto_cleanup: false,
        }
    }

    #[tokio::test]
    async fn file_upload_happy_path_runs_all_four_ssh_steps_in_order() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());

        let runner = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()), // mkdir
            Ok(ok_outcome()), // rm .part
            Ok(ok_outcome()), // scp
            Ok(ok_outcome()), // mv -n
        ]);
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let notifier = RecordingNotifier::default();

        let svc = UploadService {
            runner: Arc::new(runner.clone()),
            clipboard: Arc::new(clipboard.clone()),
            notifier: Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let out = svc.upload(&cfg).await.unwrap();
        assert!(out.clipboard_updated);

        let calls = runner.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0][0], "ssh"); // mkdir
        assert!(calls[0].last().unwrap().starts_with("mkdir -p"));
        assert_eq!(calls[1][0], "ssh"); // rm -f part
        assert!(calls[1].last().unwrap().starts_with("rm -f"));
        assert_eq!(calls[2][0], "scp"); // upload
        assert_eq!(calls[3][0], "ssh"); // mv -n
        assert!(calls[3].last().unwrap().starts_with("mv -n"));

        // Clipboard was rewritten with the final remote path.
        let writes = clipboard.written();
        assert_eq!(writes.len(), 1);
        assert!(writes[0].starts_with("/uploads/"));

        // One success notification emitted.
        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::UploadSucceeded(_)));
    }

    #[tokio::test]
    async fn clipboard_changed_mid_upload_does_not_overwrite_clipboard() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());

        let runner = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()), Ok(ok_outcome()), Ok(ok_outcome()), Ok(ok_outcome()),
        ]);
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let clipboard_clone = clipboard.clone();

        let notifier = crate::notify::fakes::RecordingNotifier::default();
        let svc = UploadService {
            runner: std::sync::Arc::new(runner.clone()),
            clipboard: std::sync::Arc::new(clipboard.clone()),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: Some(Arc::new(move || clipboard_clone.set(ClipboardContent::Other))),
        };

        let out = svc.upload(&cfg).await.unwrap();
        assert!(!out.clipboard_updated);
        assert_eq!(clipboard.written().len(), 0);
        let msgs = notifier.msgs();
        assert!(matches!(msgs[0], Message::UploadSucceededButClipboardChanged(_)));
    }

    #[tokio::test]
    async fn scp_failure_surfaces_part_path_and_does_not_touch_clipboard() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());

        let runner = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()), // mkdir
            Ok(ok_outcome()), // rm .part
            Ok(crate::ssh::runner::fakes::fail_outcome(1, "permission denied")),
        ]);
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let notifier = crate::notify::fakes::RecordingNotifier::default();

        let svc = UploadService {
            runner: std::sync::Arc::new(runner),
            clipboard: std::sync::Arc::new(clipboard.clone()),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let err = svc.upload(&cfg).await.unwrap_err();
        match err {
            UploadError::ScpFailed { stderr, part_path } => {
                assert_eq!(stderr, "permission denied");
                assert!(part_path.ends_with(".part"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(clipboard.written().len(), 0);

        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::UploadFailed { .. }));
    }

    #[tokio::test]
    async fn mv_n_refusal_yields_final_exists_error() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());

        let runner = RecordingRunner::with_scripts(vec![
            Ok(ok_outcome()),
            Ok(ok_outcome()),
            Ok(ok_outcome()),
            Ok(crate::ssh::runner::fakes::fail_outcome(1, "cannot overwrite")),
        ]);
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let notifier = crate::notify::fakes::RecordingNotifier::default();

        let svc = UploadService {
            runner: std::sync::Arc::new(runner),
            clipboard: std::sync::Arc::new(clipboard.clone()),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let err = svc.upload(&cfg).await.unwrap_err();
        assert!(matches!(err, UploadError::FinalExists(_)));

        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::FinalPathAlreadyExists(_)));
    }

    #[tokio::test]
    async fn second_upload_call_while_busy_returns_in_progress() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let guard = InFlightGuard::default();

        // Pre-acquire the guard externally to simulate an in-flight upload.
        let _held = guard.try_acquire().unwrap();

        let notifier = crate::notify::fakes::RecordingNotifier::default();
        let svc = UploadService {
            runner: std::sync::Arc::new(RecordingRunner::with_scripts(vec![])),
            clipboard: std::sync::Arc::new(clipboard),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: guard.clone(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let err = svc.upload(&cfg).await.unwrap_err();
        assert!(matches!(err, UploadError::InProgress));

        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::UploadInProgress));
    }

    #[tokio::test]
    async fn empty_clipboard_emits_notification_and_returns_error() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());
        let clipboard = FakeClipboard::new(ClipboardContent::Empty);
        let notifier = crate::notify::fakes::RecordingNotifier::default();

        let svc = UploadService {
            runner: std::sync::Arc::new(RecordingRunner::with_scripts(vec![])),
            clipboard: std::sync::Arc::new(clipboard),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let err = svc.upload(&cfg).await.unwrap_err();
        assert!(matches!(err, UploadError::ClipboardEmpty));

        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::ClipboardEmpty));
    }

    #[tokio::test]
    async fn runner_io_error_emits_io_failed_notification() {
        let key = tempfile::NamedTempFile::new().unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let cfg = valid_cfg(key.path().to_str().unwrap());
        let runner = RecordingRunner::with_scripts(vec![Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ssh missing",
        ))]);
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![file.path().to_path_buf()]));
        let notifier = crate::notify::fakes::RecordingNotifier::default();

        let svc = UploadService {
            runner: std::sync::Arc::new(runner),
            clipboard: std::sync::Arc::new(clipboard),
            notifier: std::sync::Arc::new(notifier.clone()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: tempfile::tempdir().unwrap().into_path(),
            last_uploaded: std::sync::Arc::new(std::sync::Mutex::new(None)),
            after_snapshot_hook: None,
        };

        let err = svc.upload(&cfg).await.unwrap_err();
        assert!(matches!(err, UploadError::Io(_)));

        let msgs = notifier.msgs();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::IoFailed(_)));
    }
}

#[cfg(test)]
mod local_upload_tests {
    use super::*;
    use crate::clipboard::adapter::{fakes::FakeClipboard, ClipboardContent};
    use crate::notify::fakes::RecordingNotifier;
    use crate::ssh::runner::fakes::RecordingRunner;

    fn local_svc(clipboard: FakeClipboard, output_dir: std::path::PathBuf) -> UploadService {
        UploadService {
            runner: Arc::new(RecordingRunner::with_scripts(vec![])),
            clipboard: Arc::new(clipboard),
            notifier: Arc::new(RecordingNotifier::default()),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: output_dir,
            last_uploaded: Arc::new(std::sync::Mutex::new(None)),
            #[cfg(test)]
            after_snapshot_hook: None,
        }
    }

    fn local_cfg() -> crate::config::Config {
        let mut c = crate::config::Config::default();
        c.mode = crate::config::UploadMode::Local;
        c
    }

    #[tokio::test]
    async fn local_file_upload_copies_file_and_updates_clipboard() {
        let src = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(src.path(), b"hello").unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![src.path().to_path_buf()]));
        let svc = local_svc(clipboard.clone(), output_dir.path().to_path_buf());

        let result = svc.upload(&local_cfg()).await.unwrap();

        assert!(result.clipboard_updated);
        assert!(std::path::Path::new(&result.remote_path).exists());
        let copied = std::fs::read(std::path::Path::new(&result.remote_path)).unwrap();
        assert_eq!(copied, b"hello");
        assert_eq!(clipboard.written(), vec![result.remote_path]);
    }

    #[tokio::test]
    async fn local_image_upload_writes_png_to_output_dir() {
        let output_dir = tempfile::tempdir().unwrap();
        let bytes = vec![0x89u8, 0x50, 0x4E, 0x47]; // partial PNG header
        let clipboard = FakeClipboard::new(ClipboardContent::Image(bytes.clone()));
        let svc = local_svc(clipboard.clone(), output_dir.path().to_path_buf());

        let result = svc.upload(&local_cfg()).await.unwrap();

        assert!(result.clipboard_updated);
        assert!(result.remote_path.ends_with(".png"));
        let on_disk = std::fs::read(std::path::Path::new(&result.remote_path)).unwrap();
        assert_eq!(on_disk, bytes);
    }

    #[tokio::test]
    async fn local_empty_clipboard_returns_error() {
        let output_dir = tempfile::tempdir().unwrap();
        let clipboard = FakeClipboard::new(ClipboardContent::Empty);
        let svc = local_svc(clipboard, output_dir.path().to_path_buf());

        let err = svc.upload(&local_cfg()).await.unwrap_err();
        assert!(matches!(err, UploadError::ClipboardEmpty));
    }

    #[tokio::test]
    async fn local_uploads_make_no_ssh_calls() {
        let src = tempfile::NamedTempFile::new().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let clipboard = FakeClipboard::new(ClipboardContent::Files(vec![src.path().to_path_buf()]));
        let runner = crate::ssh::runner::fakes::RecordingRunner::with_scripts(vec![]);
        let notifier = RecordingNotifier::default();
        let svc = UploadService {
            runner: Arc::new(runner.clone()),
            clipboard: Arc::new(clipboard),
            notifier: Arc::new(notifier),
            guard: InFlightGuard::default(),
            temp_dir: tempfile::tempdir().unwrap().into_path(),
            local_output_dir: output_dir.path().to_path_buf(),
            last_uploaded: Arc::new(std::sync::Mutex::new(None)),
            #[cfg(test)]
            after_snapshot_hook: None,
        };

        svc.upload(&local_cfg()).await.unwrap();
        assert_eq!(runner.calls().len(), 0, "local mode must not invoke any SSH/SCP commands");
    }
}
