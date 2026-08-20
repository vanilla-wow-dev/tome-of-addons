mod addons;
mod commands;
mod exe;
mod relocate;
mod wow;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::detect_command,
            commands::inspect_wow_exe_command,
            commands::relocate_into_command,
            commands::scan_addons_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
