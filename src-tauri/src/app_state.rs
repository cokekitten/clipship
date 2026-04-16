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

        let runner: Arc<dyn CommandRunner> = Arc::new(TokioRunner);
        let clipboard: Arc<dyn ClipboardAdapter> = Arc::new(RealClipboard);
        let notifier: Arc<dyn Notifier> = Arc::new(RealNotifier { app: app.clone() });

        let upload = UploadService {
            runner,
            clipboard,
            notifier,
            guard: InFlightGuard::default(),
            temp_dir: temp_dir.clone(),
            last_uploaded: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            after_snapshot_hook: None,
        };

        Ok(AppState { upload, config_path, temp_dir, last_shortcut_press: Mutex::new(None) })
    }
}
