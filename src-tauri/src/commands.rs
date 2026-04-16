use crate::app_state::AppState;
use crate::config::{self, Config};
use crate::config::validate::FieldWarning;
use crate::notify::Message;
use crate::test_connection;
use crate::tray;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, serde::Serialize)]
pub struct SaveConfigResponse {
    pub warnings: Vec<String>,
}

fn warning_text(w: FieldWarning) -> String {
    match w {
        FieldWarning::PrivateKeyLoosePermissions { path, mode } => {
            format!("Private key permissions look too open ({mode:o}) and OpenSSH may reject it: {path}")
        }
    }
}

/// Lazy availability check.  Returns Ok(()) if both binaries are reachable, or Err with
/// a message and emits Message::SshBinariesMissing otherwise.
pub(crate) async fn ensure_ssh_scp(state: &AppState) -> Result<(), String> {
    let a = crate::ssh::availability::check().await;
    if a.ssh && a.scp {
        Ok(())
    } else {
        state.upload.notifier.notify(Message::SshBinariesMissing);
        Err("ssh or scp missing on PATH".into())
    }
}

#[tauri::command]
pub async fn load_config(
    state: State<'_, AppState>,
) -> Result<Config, String> {
    match config::load(&state.config_path) {
        Ok(c) => Ok(c),
        Err(config::ConfigError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(Config::default())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn save_config<R: Runtime>(
    app: AppHandle<R>,
    cfg: Config,
    state: State<'_, AppState>,
) -> Result<SaveConfigResponse, String> {
    if let Err(e) = cfg.validate() {
        state.upload.notifier.notify(Message::ConfigInvalid(e.to_string()));
        return Err(e.to_string());
    }
    let warnings = cfg.warnings().into_iter().map(warning_text).collect::<Vec<_>>();
    config::save(&state.config_path, &cfg).map_err(|e| e.to_string())?;
    crate::shortcut::register(&app, &cfg.shortcut)?;
    Ok(SaveConfigResponse { warnings })
}

#[tauri::command]
pub async fn test_connection<R: Runtime>(
    cfg: Config,
    state: State<'_, AppState>,
    _app: AppHandle<R>,
) -> Result<(), String> {
    ensure_ssh_scp(&state).await?;
    let runner = state.upload.runner.clone();
    match test_connection::run(runner, &cfg).await {
        Ok(()) => Ok(()),
        Err(e) => {
            state.upload.notifier.notify(Message::MkdirFailed(e.to_string()));
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn trigger_upload_now<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    ensure_ssh_scp(&state).await?;

    let cfg = match config::load(&state.config_path) {
        Ok(c) => c,
        Err(e) => {
            state.upload.notifier.notify(Message::ConfigInvalid(e.to_string()));
            return Err(e.to_string());
        }
    };

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

#[tauri::command]
pub async fn copy_last_uploaded(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let path = state.upload.last_uploaded.lock().unwrap().clone();
    if let Some(p) = &path {
        state.upload.clipboard.write_text(p).map_err(|e| e)?;
    }
    Ok(path)
}

#[tauri::command]
pub async fn get_autostart<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}
