use crate::app_state::AppState;
use crate::commands::ensure_ssh_scp;
use crate::notify::Message;
use crate::shortcut::detect::should_fire;
use crate::tray;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub mod detect;

const DOUBLE_TAP_WINDOW_MS: u64 = 400;

/// Register the accelerator.  Returns Err(String) on failure (already notified).
pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    if let Err(e) = gs.unregister_all() {
        let msg = e.to_string();
        app.state::<AppState>()
            .upload
            .notifier
            .notify(Message::ShortcutRegistrationFailed(msg.clone()));
        return Err(msg);
    }
    let app_for_handler = app.clone();
    match gs.on_shortcut(accelerator, move |_app, _shortcut, event| {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        let state = app_for_handler.state::<AppState>();
        let cfg = match crate::config::load(&state.config_path) {
            Ok(c) => c,
            Err(_) => {
                // If config is unreadable, fall back to immediate fire; the
                // upload path itself will surface a ConfigInvalid notification.
                tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
                return;
            }
        };

        if !cfg.shortcut_double_tap {
            tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
            return;
        }

        let mut guard = state.last_shortcut_press.lock().unwrap();
        let (fire, next) = should_fire(
            *guard,
            Instant::now(),
            Duration::from_millis(DOUBLE_TAP_WINDOW_MS),
        );
        *guard = next;
        drop(guard);

        if fire {
            tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
        }
    }) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            app.state::<AppState>()
                .upload
                .notifier
                .notify(Message::ShortcutRegistrationFailed(msg.clone()));
            Err(msg)
        }
    }
}

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
