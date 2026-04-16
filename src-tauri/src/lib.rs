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
                        Err(e) => {
                            eprintln!("cleanup loop: failed to load config: {e}");
                            continue;
                        }
                    };
                    if !cfg.auto_cleanup {
                        continue;
                    }
                    let local_dir = state.upload.local_output_dir.clone();
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

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}
