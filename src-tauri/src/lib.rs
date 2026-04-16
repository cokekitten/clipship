pub mod naming;
pub mod config;
pub mod ssh;
pub mod clipboard;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}
