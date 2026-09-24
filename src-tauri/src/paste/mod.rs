use crate::config::Config;
use crate::upload::errors::UploadError;
use crate::upload::service::UploadSuccess;

/// Paste only when the user opted in AND the path actually reached the clipboard.
pub fn should_paste(cfg: &Config, result: &Result<UploadSuccess, UploadError>) -> bool {
    cfg.auto_paste && matches!(result, Ok(s) if s.clipboard_updated)
}

/// Sleep briefly (the hotkey may still be held), then send the paste keystroke.
pub async fn paste_after_delay() -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    send_paste()
}

fn send_paste() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // Release modifiers the hotkey may still hold so the paste is not seen as e.g. Cmd+Shift+V.
    for key in [Key::Shift, Key::Control, Key::Alt, Key::Meta] {
        let _ = enigo.key(key, Direction::Release);
    }
    #[cfg(target_os = "macos")]
    let primary = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let primary = Key::Control;
    enigo
        .key(primary, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(primary, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UploadMode;

    fn cfg(auto_paste: bool) -> Config {
        Config {
            version: 1,
            mode: UploadMode::Local,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
            auto_cleanup: false,
            auto_paste,
        }
    }

    fn success(clipboard_updated: bool) -> UploadSuccess {
        UploadSuccess {
            remote_path: "/tmp/x.png".into(),
            clipboard_updated,
        }
    }

    #[test]
    fn off_never_pastes_even_on_success() {
        assert!(!should_paste(&cfg(false), &Ok(success(true))));
    }

    #[test]
    fn on_and_clipboard_updated_pastes() {
        assert!(should_paste(&cfg(true), &Ok(success(true))));
    }

    #[test]
    fn on_but_clipboard_changed_does_not_paste() {
        assert!(!should_paste(&cfg(true), &Ok(success(false))));
    }

    #[test]
    fn on_but_upload_failed_does_not_paste() {
        assert!(!should_paste(&cfg(true), &Err(UploadError::ClipboardEmpty)));
    }
}
