use tauri::{AppHandle, Runtime};

/// Stub replaced in Task 19.  Calling it with no tray icon registered is a no-op.
pub fn set_status<R: Runtime>(_app: &AppHandle<R>, _label: &str) {}

/// Stub replaced in Task 19.
pub fn set_last_uploaded_enabled<R: Runtime>(_app: &AppHandle<R>, _enabled: bool) {}

/// Stub replaced in Task 19.
pub fn init<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> { Ok(()) }
