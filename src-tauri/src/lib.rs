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

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app_state::AppState::build(handle.clone())?;
            app.manage(state);

            // Register shortcut from saved config if one exists.
            if let Ok(cfg) = config::load(&app.state::<app_state::AppState>().config_path.clone()) {
                let _ = shortcut::register(&handle, &cfg.shortcut);
            }

            tray::init(&handle)?;

            // Hide settings window on close instead of quitting.
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
            commands::test_connection,
            commands::trigger_upload_now,
            commands::copy_last_uploaded,
        ])
        .run(tauri::generate_context!())
        .expect("error running Clipship");
}

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}
