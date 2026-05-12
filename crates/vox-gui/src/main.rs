#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::catalog::get_command_catalog,
            commands::execute::execute_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
