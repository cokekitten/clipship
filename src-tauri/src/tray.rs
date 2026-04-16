use crate::app_state::AppState;
use crate::commands::ensure_ssh_scp;
use crate::notify::Message;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

/// Holds tray menu item references so set_status / set_last_uploaded_enabled can
/// update them without storing a generic `Menu<R>` in AppState.
pub struct TrayItems<R: Runtime> {
    pub status: MenuItem<R>,
    pub copy_last: MenuItem<R>,
}

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
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    // Store references so callers can update status text / enable copy_last.
    app.manage(TrayItems { status, copy_last });

    Ok(())
}

fn handle_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        "upload_now" => spawn_upload(app.clone()),
        "settings" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "copy_last" => {
            let state = app.state::<AppState>();
            let last = state.upload.last_uploaded.lock().unwrap().clone();
            if let Some(p) = last {
                let _ = state.upload.clipboard.write_text(&p);
            }
        }
        "quit" => spawn_quit(app.clone()),
        _ => {}
    }
}

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

fn spawn_quit<R: Runtime>(app: AppHandle<R>) {
    let state = app.state::<AppState>();
    if !state.upload.guard.is_busy() {
        app.exit(0);
        return;
    }
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_dialog::DialogExt;
        let confirm = app
            .dialog()
            .message("An upload is in progress. Quit anyway? The remote .part file may be left behind.")
            .title("Quit Clipship")
            .blocking_show();
        if confirm {
            app.state::<AppState>()
                .upload
                .notifier
                .notify(Message::QuitDuringUpload);
            app.exit(0);
        }
    });
}

pub fn set_status<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(items) = app.try_state::<TrayItems<R>>() {
        let _ = items.status.set_text(label);
    }
}

pub fn set_last_uploaded_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    if let Some(items) = app.try_state::<TrayItems<R>>() {
        let _ = items.copy_last.set_enabled(enabled);
    }
}
