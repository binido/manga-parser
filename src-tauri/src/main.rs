#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod session;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(session::Session::default())
        .invoke_handler(tauri::generate_handler![
            commands::prepare,
            commands::cancel
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить приложение");
}
