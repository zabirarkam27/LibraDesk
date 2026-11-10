#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod storage;

fn main() {
    tauri::Builder::default()
        .manage(commands::State::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_preferences,
            commands::set_theme
        ])
        .run(tauri::generate_context!())
        .expect("failed to start LibraDesk");
}
