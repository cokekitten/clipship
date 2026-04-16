# Local Mode + Auto-Cleanup + UI Refinements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local-save upload mode, an optional hourly auto-cleanup, and several settings UI improvements (header save button, mode toggle, switch-row fixes, scrollbar hiding, tray cleanup).

**Architecture:** `Config` gains `mode: UploadMode` and `auto_cleanup: bool`. `UploadService::upload_inner` dispatches to `upload_via_ssh` (existing logic) or `upload_via_local` (new). A background cleanup loop spawned at startup re-reads config on each hourly tick. Tray loses the Test Connection item. The settings UI gains a mode segmented-button, repositioned Save, dimmed SSH fields in local mode, and corrected switch-row behaviour.

**Tech Stack:** Rust / Tauri 2, Tokio, Serde JSON, Svelte 5, Tailwind CSS v3 + shadcn-svelte, Vitest.

---

## File Map

**Create:**
- `src-tauri/src/cleanup/mod.rs` — `is_ssh_complete`, `cleanup_local`, `cleanup_remote`

**Modify:**
- `src-tauri/src/config/mod.rs` — add `UploadMode` enum + two fields, conditional `validate()`
- `src-tauri/src/upload/service.rs` — add `local_output_dir`, split upload into `upload_via_ssh` + `upload_via_local`
- `src-tauri/src/app_state.rs` — set `local_output_dir` on `UploadService`
- `src-tauri/src/commands.rs` — skip SSH binary check when `mode == Local`
- `src-tauri/src/shortcut/mod.rs` — skip SSH binary check when `mode == Local`
- `src-tauri/src/tray.rs` — skip SSH binary check when `mode == Local`; remove Test Connection item
- `src-tauri/src/ssh/commands.rs` — add `find_and_delete_old`
- `src-tauri/src/lib.rs` — `pub mod cleanup`, spawn cleanup background loop
- `src/lib/types.ts` — add `mode` and `auto_cleanup` to `Config`
- `src/App.svelte` — header with Save, mode toggle, SSH dimming, System card additions
- `src/components/ShortcutSection.svelte` — fix switch-row click scope
- `src/app.css` — hide scrollbar globally

---

## Task 1: Config — UploadMode enum, new fields, conditional validate

**Files:**
- Modify: `src-tauri/src/config/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to the bottom of the existing `#[cfg(test)] mod tests` block inside `src-tauri/src/config/mod.rs`:

```rust
    #[test]
    fn mode_defaults_to_ssh_when_field_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        ).unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.mode, UploadMode::Ssh);
    }

    #[test]
    fn auto_cleanup_defaults_to_false_when_field_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        ).unwrap();
        let cfg = load(&path).unwrap();
        assert!(!cfg.auto_cleanup);
    }

    #[test]
    fn local_mode_validate_skips_ssh_fields() {
        let mut cfg = Config::default();
        cfg.mode = UploadMode::Local;
        // host/username/etc. are all empty — should not fail in local mode
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn ssh_mode_validate_still_requires_host() {
        let cfg = Config::default(); // mode == Ssh, host == ""
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.field, "host");
    }

    #[test]
    fn mode_and_auto_cleanup_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.mode = UploadMode::Local;
        cfg.auto_cleanup = true;
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.mode, UploadMode::Local);
        assert!(back.auto_cleanup);
    }
```

- [ ] **Step 2: Run tests — expect compile error**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test 2>&1 | head -30
```

Expected: compile error `cannot find value 'UploadMode'`.

- [ ] **Step 3: Implement UploadMode enum and new Config fields**

Replace the top of `src-tauri/src/config/mod.rs` (everything before `impl Default for Config`) with:

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod validate;

const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    #[default]
    Ssh,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub mode: UploadMode,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: String,
    pub remote_dir: String,
    pub shortcut: String,
    #[serde(default)]
    pub shortcut_double_tap: bool,
    #[serde(default)]
    pub auto_cleanup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            mode: UploadMode::Ssh,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
            auto_cleanup: false,
        }
    }
}
```

- [ ] **Step 4: Update `Config::validate()` to be conditional on mode**

Replace the `validate()` method body:

```rust
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.mode == UploadMode::Ssh {
            validate::host(&self.host).map_err(|e| ValidationError { field: "host", err: e })?;
            validate::port(self.port as u32).map_err(|e| ValidationError { field: "port", err: e })?;
            validate::username(&self.username).map_err(|e| ValidationError { field: "username", err: e })?;
            validate::private_key_path(&self.private_key_path).map_err(|e| ValidationError { field: "private_key_path", err: e })?;
            validate::remote_dir(&self.remote_dir).map_err(|e| ValidationError { field: "remote_dir", err: e })?;
        }
        validate::shortcut(&self.shortcut).map_err(|e| ValidationError { field: "shortcut", err: e })?;
        Ok(())
    }
```

- [ ] **Step 5: Run tests — expect all pass**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test config::
```

Expected: all config tests pass, including the 5 new ones.

- [ ] **Step 6: Commit**

```bash
cd src-tauri && git add src/config/mod.rs
cd .. && git commit -m "feat(config): add UploadMode enum, mode and auto_cleanup fields, conditional validate"
```

---

## Task 2: UploadService — local_output_dir field + upload_via_local

**Files:**
- Modify: `src-tauri/src/upload/service.rs`
- Modify: `src-tauri/src/app_state.rs`

### 2a — App state wiring

- [ ] **Step 1: Add `local_output_dir` to `UploadService` struct**

In `src-tauri/src/upload/service.rs`, add the field after `temp_dir`:

```rust
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
```

- [ ] **Step 2: Wire `local_output_dir` in `AppState::build()`**

In `src-tauri/src/app_state.rs`, add `local_output_dir` to `UploadService` construction:

```rust
use crate::clipboard::adapter::{ClipboardAdapter, RealClipboard};
use tauri::Manager;
use crate::notify::{Notifier, RealNotifier};
use crate::ssh::runner::{CommandRunner, TokioRunner};
use crate::upload::{guard::InFlightGuard, service::UploadService};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub upload: UploadService,
    pub config_path: PathBuf,
    pub temp_dir: PathBuf,
    pub last_shortcut_press: Mutex<Option<std::time::Instant>>,
}

impl AppState {
    pub fn build<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> anyhow::Result<Self> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|e| anyhow::anyhow!("no app_config_dir: {e}"))?;
        let config_path = crate::config::config_file(&config_dir);
        let temp_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| anyhow::anyhow!("no app_local_data_dir: {e}"))?
            .join("clipship-tmp");

        let local_output_dir = std::env::temp_dir().join("clipship");

        let runner: Arc<dyn CommandRunner> = Arc::new(TokioRunner);
        let clipboard: Arc<dyn ClipboardAdapter> = Arc::new(RealClipboard);
        let notifier: Arc<dyn Notifier> = Arc::new(RealNotifier { app: app.clone() });

        let upload = UploadService {
            runner,
            clipboard,
            notifier,
            guard: InFlightGuard::default(),
            temp_dir: temp_dir.clone(),
            local_output_dir,
            last_uploaded: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            after_snapshot_hook: None,
        };

        Ok(AppState { upload, config_path, temp_dir, last_shortcut_press: Mutex::new(None) })
    }
}
```

- [ ] **Step 3: Verify it compiles**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo build 2>&1 | head -40
```

Expected: compile errors only about missing fields in test UploadService constructions (not a logic error).

### 2b — Refactor upload_inner + add upload_via_local

- [ ] **Step 4: Write failing tests for local upload**

Add a new test module at the bottom of `src-tauri/src/upload/service.rs`, after `happy_path_tests`:

```rust
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
```

- [ ] **Step 5: Run tests — expect compile error**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test upload::local_upload_tests 2>&1 | head -30
```

Expected: compile error about missing `local_output_dir` in existing test constructions, or `UploadMode` not in scope.

- [ ] **Step 6: Fix all existing test UploadService constructions**

In `src-tauri/src/upload/service.rs`, find every `UploadService {` in the `happy_path_tests` module and add `local_output_dir: tempfile::tempdir().unwrap().into_path(),` after the `temp_dir` field. There are 7 occurrences. Also update `valid_cfg` to add the new Config fields:

```rust
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
```

- [ ] **Step 7: Refactor upload_inner and add upload methods**

Replace the entire `impl UploadService` block (not counting the test modules) in `service.rs` with:

```rust
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

        let remote_name = filename::build_remote_filename(&original_name);
        let remote_final = remote_path::join(&cfg.remote_dir, &remote_name);
        let remote_part = remote_path::part_path(&remote_final);

        let snap = Snapshot::of(&content);

        #[cfg(test)]
        if let Some(h) = &self.after_snapshot_hook {
            h();
        }

        let mkdir_argv = commands::mkdir(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &cfg.remote_dir,
        );
        let out = self.runner.run(mkdir_argv).await?;
        if !out.success {
            return Err(UploadError::MkdirFailed(out.stderr));
        }

        let rm_argv = commands::rm_part(
            cfg.port, &cfg.private_key_path, &cfg.username, &cfg.host, &remote_part,
        );
        let _ = self.runner.run(rm_argv).await?;

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
```

Also add `ClipboardContent` to the existing imports at the top of `service.rs`:

```rust
use crate::clipboard::adapter::{ClipboardAdapter, ClipboardContent};
```

- [ ] **Step 8: Run all upload tests — expect pass**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test upload::
```

Expected: all existing SSH tests + 4 new local tests pass.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/upload/service.rs src-tauri/src/app_state.rs
git commit -m "feat(upload): add local mode – copy clipboard to tmp/clipship instead of SSH"
```

---

## Task 3: Skip SSH binary check in local mode

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/shortcut/mod.rs`
- Modify: `src-tauri/src/tray.rs`

No new tests (integration-level behaviour; SSH tests continue to pass unmodified).

- [ ] **Step 1: Update `trigger_upload_now` in commands.rs**

Replace the body of `trigger_upload_now` with:

```rust
#[tauri::command]
pub async fn trigger_upload_now<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let cfg = match config::load(&state.config_path) {
        Ok(c) => c,
        Err(e) => {
            state.upload.notifier.notify(Message::ConfigInvalid(e.to_string()));
            return Err(e.to_string());
        }
    };

    if cfg.mode == crate::config::UploadMode::Ssh {
        ensure_ssh_scp(&state).await?;
    }

    tray::set_status(&app, "Uploading\u{2026}");
    let result = state.upload.upload(&cfg).await;
    tray::set_status(&app, "Idle");
    if state.upload.last_uploaded.lock().unwrap().is_some() {
        tray::set_last_uploaded_enabled(&app, true);
    }

    match result {
        Ok(s) => Ok(s.remote_path),
        Err(e) => Err(e.to_string()),
    }
}
```

- [ ] **Step 2: Update `run_shortcut_upload` in shortcut/mod.rs**

Replace the `run_shortcut_upload` function:

```rust
async fn run_shortcut_upload<R: Runtime>(app: AppHandle<R>) {
    let state = app.state::<AppState>();
    let cfg = match crate::config::load(&state.config_path) {
        Ok(c) => c,
        Err(e) => {
            state.upload.notifier.notify(Message::ConfigInvalid(e.to_string()));
            return;
        }
    };
    if cfg.mode == crate::config::UploadMode::Ssh && ensure_ssh_scp(&state).await.is_err() {
        return;
    }
    tray::set_status(&app, "Uploading\u{2026}");
    let _ = state.upload.upload(&cfg).await;
    tray::set_status(&app, "Idle");
    if state.upload.last_uploaded.lock().unwrap().is_some() {
        tray::set_last_uploaded_enabled(&app, true);
    }
}
```

- [ ] **Step 3: Update `spawn_upload` in tray.rs**

Replace `spawn_upload`:

```rust
fn spawn_upload<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let cfg = match crate::config::load(&state.config_path) {
            Ok(c) => c,
            Err(e) => {
                state.upload.notifier.notify(Message::ConfigInvalid(e.to_string()));
                return;
            }
        };
        if cfg.mode == crate::config::UploadMode::Ssh && ensure_ssh_scp(&state).await.is_err() {
            return;
        }
        set_status(&app, "Uploading\u{2026}");
        let _ = state.upload.upload(&cfg).await;
        set_status(&app, "Idle");
        if state.upload.last_uploaded.lock().unwrap().is_some() {
            set_last_uploaded_enabled(&app, true);
        }
    });
}
```

- [ ] **Step 4: Verify it compiles and tests still pass**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test 2>&1 | tail -10
```

Expected: all tests pass, no compile errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/shortcut/mod.rs src-tauri/src/tray.rs
git commit -m "fix: skip SSH binary check when upload mode is local"
```

---

## Task 4: SSH cleanup command + cleanup module + background loop

**Files:**
- Modify: `src-tauri/src/ssh/commands.rs`
- Create: `src-tauri/src/cleanup/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `find_and_delete_old` to ssh/commands.rs**

Append to `src-tauri/src/ssh/commands.rs` (before the `#[cfg(test)]` block):

```rust
pub fn find_and_delete_old(port: u16, key: &str, user: &str, host: &str, remote_dir: &str) -> Vec<String> {
    let mut v = ssh_base(port, key, user, host);
    v.push(format!(
        "find '{}' -maxdepth 1 -mtime +7 -type f -delete",
        remote_dir
    ));
    v
}
```

Also add a test inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn find_and_delete_old_shape() {
        let v = find_and_delete_old(22, "/k", "u", "h.com", "/r");
        assert_eq!(v[0], SSH_BIN);
        assert!(v.last().unwrap().contains("find '/r' -maxdepth 1 -mtime +7 -type f -delete"));
    }
```

- [ ] **Step 2: Run ssh command tests**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test ssh::commands::tests
```

Expected: all pass including `find_and_delete_old_shape`.

- [ ] **Step 3: Write failing cleanup tests**

Create `src-tauri/src/cleanup/mod.rs` with tests only (no implementation yet):

```rust
use crate::config::Config;
use crate::ssh::runner::CommandRunner;
use std::path::Path;
use std::time::Duration;

pub fn is_ssh_complete(cfg: &Config) -> bool {
    todo!()
}

pub fn cleanup_local(dir: &Path, max_age: Duration) {
    todo!()
}

pub async fn cleanup_remote(cfg: &Config, runner: &dyn CommandRunner) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn is_ssh_complete_false_when_any_field_empty() {
        let cfg = Config::default();
        assert!(!is_ssh_complete(&cfg));
    }

    #[test]
    fn is_ssh_complete_true_when_all_fields_set() {
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.username = "u".into();
        cfg.private_key_path = "/k".into();
        cfg.remote_dir = "/r".into();
        assert!(is_ssh_complete(&cfg));
    }

    #[test]
    fn cleanup_local_noop_when_dir_absent() {
        let dir = tempdir().unwrap();
        let nonexistent = dir.path().join("nope");
        // should not panic
        cleanup_local(&nonexistent, Duration::from_secs(1));
    }

    #[test]
    fn cleanup_local_deletes_files_older_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("old.txt");
        std::fs::write(&file, b"x").unwrap();
        // Use 0 duration so the file (written just now) counts as "old"
        std::thread::sleep(std::time::Duration::from_millis(1));
        cleanup_local(dir.path(), Duration::from_millis(0));
        assert!(!file.exists(), "file should have been deleted");
    }

    #[test]
    fn cleanup_local_keeps_files_newer_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new.txt");
        std::fs::write(&file, b"x").unwrap();
        cleanup_local(dir.path(), Duration::from_secs(3600));
        assert!(file.exists(), "file should have been kept");
    }
}
```

- [ ] **Step 4: Run cleanup tests — expect compile error (todo! panics)**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test cleanup:: 2>&1 | head -20
```

Expected: tests compile but panic on `todo!()`.

- [ ] **Step 5: Implement cleanup module**

Replace the stubs in `src-tauri/src/cleanup/mod.rs` with full implementations:

```rust
use crate::config::Config;
use crate::ssh::runner::CommandRunner;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub fn is_ssh_complete(cfg: &Config) -> bool {
    !cfg.host.is_empty()
        && !cfg.username.is_empty()
        && !cfg.private_key_path.is_empty()
        && !cfg.remote_dir.is_empty()
}

/// Delete files in `dir` older than `max_age`. Silently skips unreadable entries.
pub fn cleanup_local(dir: &Path, max_age: Duration) {
    if !dir.exists() {
        return;
    }
    let threshold = match SystemTime::now().checked_sub(max_age) {
        Some(t) => t,
        None => return,
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cleanup_local read_dir {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let modified = match meta.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if modified < threshold {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                eprintln!("cleanup_local remove {}: {e}", entry.path().display());
            }
        }
    }
}

/// Run `find <remote_dir> -maxdepth 1 -mtime +7 -type f -delete` via SSH.
/// No-op when SSH config is incomplete. Errors are logged, not propagated.
pub async fn cleanup_remote(cfg: &Config, runner: &dyn CommandRunner) {
    if !is_ssh_complete(cfg) {
        return;
    }
    let argv = crate::ssh::commands::find_and_delete_old(
        cfg.port,
        &cfg.private_key_path,
        &cfg.username,
        &cfg.host,
        &cfg.remote_dir,
    );
    match runner.run(argv).await {
        Ok(out) if !out.success => {
            eprintln!("cleanup_remote: remote find/delete failed: {}", out.stderr);
        }
        Err(e) => {
            eprintln!("cleanup_remote: ssh error: {e}");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    // (tests written in Step 3 remain here unchanged)
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn is_ssh_complete_false_when_any_field_empty() {
        let cfg = Config::default();
        assert!(!is_ssh_complete(&cfg));
    }

    #[test]
    fn is_ssh_complete_true_when_all_fields_set() {
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.username = "u".into();
        cfg.private_key_path = "/k".into();
        cfg.remote_dir = "/r".into();
        assert!(is_ssh_complete(&cfg));
    }

    #[test]
    fn cleanup_local_noop_when_dir_absent() {
        let dir = tempdir().unwrap();
        let nonexistent = dir.path().join("nope");
        cleanup_local(&nonexistent, Duration::from_secs(1));
    }

    #[test]
    fn cleanup_local_deletes_files_older_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("old.txt");
        std::fs::write(&file, b"x").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cleanup_local(dir.path(), Duration::from_millis(0));
        assert!(!file.exists(), "file should have been deleted");
    }

    #[test]
    fn cleanup_local_keeps_files_newer_than_max_age() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new.txt");
        std::fs::write(&file, b"x").unwrap();
        cleanup_local(dir.path(), Duration::from_secs(3600));
        assert!(file.exists(), "file should have been kept");
    }
}
```

- [ ] **Step 6: Register cleanup module and spawn background loop in lib.rs**

In `src-tauri/src/lib.rs`, add `pub mod cleanup;` to the module declarations, and add the background loop inside `setup()`, after `tray::init(&handle)?;`:

```rust
pub mod naming;
pub mod config;
pub mod ssh;
pub mod clipboard;
pub mod notify;
pub mod upload;
pub mod test_connection;
pub mod app_state;
pub mod tray;
pub mod commands;
pub mod shortcut;
pub mod cleanup;

use std::time::Duration;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let state = app_state::AppState::build(handle.clone())?;
            app.manage(state);

            if let Ok(cfg) = config::load(&app.state::<app_state::AppState>().config_path.clone()) {
                let _ = shortcut::register(&handle, &cfg.shortcut);
            }

            tray::init(&handle)?;

            // Background auto-cleanup loop: runs every hour, re-reads config each tick.
            let cleanup_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(3600));
                interval.tick().await; // discard the immediate first tick
                loop {
                    interval.tick().await;
                    let state = cleanup_handle.state::<app_state::AppState>();
                    let cfg = match config::load(&state.config_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if !cfg.auto_cleanup {
                        continue;
                    }
                    let local_dir = std::env::temp_dir().join("clipship");
                    cleanup::cleanup_local(&local_dir, Duration::from_secs(7 * 24 * 3600));
                    let runner = state.upload.runner.clone();
                    cleanup::cleanup_remote(&cfg, runner.as_ref()).await;
                }
            });

            if let Some(w) = app.get_webview_window("main") {
                let w_clone = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::default_private_key,
            commands::test_connection,
            commands::trigger_upload_now,
            commands::copy_last_uploaded,
            commands::get_autostart,
            commands::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error running Clipship");
}
```

- [ ] **Step 7: Run all tests — expect pass**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo test 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/cleanup/mod.rs src-tauri/src/ssh/commands.rs src-tauri/src/lib.rs
git commit -m "feat(cleanup): add hourly auto-cleanup for local tmp and remote SSH dirs"
```

---

## Task 5: Tray — remove Test Connection item

**Files:**
- Modify: `src-tauri/src/tray.rs`

- [ ] **Step 1: Remove test_conn item, its menu slot, and spawn_test**

In `src-tauri/src/tray.rs`:

1. Delete the line: `let test_conn = MenuItem::with_id(app, "test", "Test connection", true, None::<&str>)?;`
2. Remove `&test_conn,` from `Menu::with_items`.
3. In `handle_event`, remove the arm: `"test" => spawn_test(app.clone()),`
4. Delete the entire `fn spawn_test<R: Runtime>(...)` function.

The `init` function becomes:

```rust
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let upload_now = MenuItem::with_id(app, "upload_now", "Upload clipboard now", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Open settings", true, None::<&str>)?;
    let copy_last = MenuItem::with_id(app, "copy_last", "Copy last uploaded path", false, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Idle", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &upload_now,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &copy_last,
            &status,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("clipship-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| handle_event(app, event))
        .build(app)?;

    app.manage(TrayItems { status, copy_last });

    Ok(())
}
```

- [ ] **Step 2: Verify compile**

```
cd src-tauri && PATH="$USERPROFILE/.cargo/bin:$PATH" cargo build 2>&1 | tail -5
```

Expected: clean build, no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tray.rs
git commit -m "fix(tray): remove Test Connection menu item"
```

---

## Task 6: Frontend types

**Files:**
- Modify: `src/lib/types.ts`

No test (type-only change; TypeScript compiler catches misuse at build time).

- [ ] **Step 1: Update Config interface**

Replace `src/lib/types.ts` with:

```ts
export interface Config {
  version: 1;
  mode: "ssh" | "local";
  host: string;
  port: number;
  username: string;
  private_key_path: string;
  remote_dir: string;
  shortcut: string;
  shortcut_double_tap: boolean;
  auto_cleanup: boolean;
}

export interface Status {
  kind: "idle" | "ok" | "error";
  message: string;
  detail?: string;
}

export interface SaveConfigResponse {
  warnings: string[];
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```
pnpm tsc --noEmit 2>&1 | head -20
```

Expected: no errors (or only pre-existing errors unrelated to types.ts).

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(types): add mode and auto_cleanup to Config interface"
```

---

## Task 7: App.svelte — header, mode toggle, SSH dimming, test-conn conditional

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Rewrite App.svelte**

Replace the entire `src/App.svelte` with:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import type { Config, Status } from "./lib/types";
  import {
    loadConfig,
    saveConfig,
    testConnection,
    getAutostart,
    setAutostart,
    defaultPrivateKey,
  } from "./lib/bridge";
  import SshSection from "./components/SshSection.svelte";
  import DestinationSection from "./components/DestinationSection.svelte";
  import ShortcutSection from "./components/ShortcutSection.svelte";
  import StatusArea from "./components/StatusArea.svelte";
  import * as Card from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";
  import { Button } from "$lib/components/ui/button";

  let cfg: Config = $state({
    version: 1,
    mode: "ssh",
    host: "",
    port: 22,
    username: "",
    private_key_path: "",
    remote_dir: "",
    shortcut: "CmdOrCtrl+Shift+U",
    shortcut_double_tap: false,
    auto_cleanup: false,
  });

  let status: Status = $state({ kind: "idle", message: "" });
  let autostart: boolean = $state(false);

  onMount(async () => {
    try {
      cfg = await loadConfig();
    } catch (e) {
      status = { kind: "error", message: "Failed to load configuration", detail: String(e) };
    }
    if (!cfg.private_key_path) {
      try {
        const k = await defaultPrivateKey();
        if (k) cfg.private_key_path = k;
      } catch (_) {}
    }
    try {
      autostart = await getAutostart();
    } catch (_) {}
  });

  async function onSave() {
    try {
      const result = await saveConfig(cfg);
      status = result.warnings.length
        ? { kind: "ok", message: "Saved with warnings.", detail: result.warnings.join("\n") }
        : { kind: "ok", message: "Saved." };
    } catch (e) {
      status = { kind: "error", message: "Save failed", detail: String(e) };
    }
  }

  async function onTest() {
    status = { kind: "idle", message: "Testing…" };
    try {
      await testConnection(cfg);
      status = { kind: "ok", message: "Connection OK." };
    } catch (e) {
      status = { kind: "error", message: "Test failed", detail: String(e) };
    }
  }

  async function onAutostartChange(v: boolean) {
    autostart = v;
    try {
      await setAutostart(autostart);
    } catch (e) {
      status = { kind: "error", message: "Failed to update autostart", detail: String(e) };
      autostart = !autostart;
    }
  }
</script>

<main class="mx-auto flex max-w-2xl flex-col gap-4 p-6">
  <!-- Header row: title + save button -->
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-semibold">Clipship</h1>
    <Button onclick={onSave}>Save</Button>
  </div>

  <!-- Mode toggle -->
  <div class="flex gap-1 rounded-md border p-1 w-fit">
    <Button
      variant={cfg.mode === "ssh" ? "default" : "ghost"}
      size="sm"
      onclick={() => (cfg.mode = "ssh")}
    >SSH</Button>
    <Button
      variant={cfg.mode === "local" ? "default" : "ghost"}
      size="sm"
      onclick={() => (cfg.mode = "local")}
    >Local</Button>
  </div>

  <!-- SSH + Destination sections: dimmed in local mode -->
  <div class={cfg.mode === "local" ? "pointer-events-none opacity-50" : ""}>
    <div class="flex flex-col gap-4">
      <SshSection bind:cfg />
      <DestinationSection bind:cfg />
    </div>
  </div>

  <!-- Test connection: SSH mode only -->
  {#if cfg.mode === "ssh"}
    <div>
      <Button variant="secondary" onclick={onTest}>Test connection</Button>
    </div>
  {/if}

  <ShortcutSection bind:cfg />

  <!-- System card -->
  <Card.Root>
    <Card.Header>
      <Card.Title>System</Card.Title>
    </Card.Header>
    <Card.Content class="grid gap-4">
      <!-- Launch at login -->
      <div class="flex items-center justify-between">
        <div class="flex flex-col gap-1">
          <span class="text-sm font-medium">Launch at login</span>
          <span class="text-xs text-muted-foreground">
            Start Clipship automatically when you sign in.
          </span>
        </div>
        <Switch checked={autostart} onCheckedChange={onAutostartChange} />
      </div>
      <!-- Auto-cleanup -->
      <div class="flex items-center justify-between">
        <div class="flex flex-col gap-1">
          <span class="text-sm font-medium">Auto-cleanup</span>
          <span class="text-xs text-muted-foreground">
            Delete files older than 7 days every hour. Remote cleanup requires SSH config to be complete.
          </span>
        </div>
        <Switch
          checked={cfg.auto_cleanup}
          onCheckedChange={(v) => (cfg.auto_cleanup = v)}
        />
      </div>
    </Card.Content>
  </Card.Root>

  <StatusArea {status} />
</main>
```

- [ ] **Step 2: Verify TypeScript/Svelte compiles**

```
pnpm tsc --noEmit 2>&1 | head -20
```

Expected: no new errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat(ui): mode toggle, repositioned save button, SSH dimming, auto-cleanup toggle"
```

---

## Task 8: Switch row click fix + scrollbar hiding

**Files:**
- Modify: `src/components/ShortcutSection.svelte`
- Modify: `src/app.css`

- [ ] **Step 1: Fix switch row in ShortcutSection.svelte**

Replace the double-tap row (the `<div class="flex items-center justify-between">` block inside `Card.Content`) with a version that removes the `<Label>` component (which has `for=` making the whole label clickable):

```svelte
<Card.Root>
  <Card.Header>
    <Card.Title>Shortcut</Card.Title>
  </Card.Header>
  <Card.Content class="grid gap-4">
    <div class="grid gap-1.5">
      <span class="text-sm font-medium">Global shortcut</span>
      <ShortcutRecorder value={cfg.shortcut} onChange={onAccel} />
    </div>
    <div class="flex items-center justify-between">
      <div class="flex flex-col gap-1">
        <span class="text-sm font-medium">Require double-tap to trigger</span>
        <span class="text-xs text-muted-foreground">
          Press the shortcut twice within 400&nbsp;ms to upload.
        </span>
      </div>
      <Switch
        checked={cfg.shortcut_double_tap}
        onCheckedChange={onDoubleTapChange}
      />
    </div>
  </Card.Content>
</Card.Root>
```

Also remove the unused `Label` import since `<Label>` is no longer used:

```svelte
<script lang="ts">
  import type { Config } from "../lib/types";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";
  import * as Card from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";

  let { cfg = $bindable() }: { cfg: Config } = $props();

  function onAccel(v: string) {
    cfg.shortcut = v;
  }

  function onDoubleTapChange(v: boolean) {
    cfg.shortcut_double_tap = v;
  }
</script>
```

- [ ] **Step 2: Hide scrollbar globally in app.css**

Add to the end of `src/app.css`:

```css
/* Hide scrollbar on all elements */
* {
  scrollbar-width: none; /* Firefox */
}
*::-webkit-scrollbar {
  display: none; /* Chrome / Safari / Edge */
}
```

- [ ] **Step 3: Verify compile**

```
pnpm tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/ShortcutSection.svelte src/app.css
git commit -m "fix(ui): remove label-click-triggers-switch, hide scrollbar, left-align switch rows"
```

---

## Self-Review

**Spec coverage check:**
- ✅ `UploadMode` enum + `mode` field + `auto_cleanup` field (Task 1)
- ✅ `validate()` only checks SSH fields when `mode == Ssh` (Task 1)
- ✅ Local upload: classify → copy to `tmp/clipship` → clipboard update (Task 2)
- ✅ SSH binary check skipped in local mode (Task 3)
- ✅ `cleanup_local` + `cleanup_remote` + `is_ssh_complete` (Task 4)
- ✅ Background hourly loop with config re-read (Task 4)
- ✅ Tray: Test Connection removed (Task 5)
- ✅ Frontend types: `mode` + `auto_cleanup` (Task 6)
- ✅ Save button in header row (Task 7)
- ✅ Mode toggle (Task 7)
- ✅ SSH sections dimmed in local mode (Task 7)
- ✅ Test connection button SSH-only (Task 7)
- ✅ Auto-cleanup toggle in System card (Task 7)
- ✅ Switch rows: only switch clickable, left-aligned (Task 8)
- ✅ Scrollbar hidden (Task 8)

**Type consistency check:** `UploadMode::Ssh/Local` used consistently across Tasks 1-4. `cfg.mode === "ssh"/"local"` in frontend matches `#[serde(rename_all = "snake_case")]` on the Rust enum. `local_output_dir: PathBuf` added to both `UploadService` (Task 2) and `AppState::build()` (Task 2).
