#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod metadata;
mod scan;
mod storage;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::State::default())
        .setup(|app| {
            scan::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_preferences,
            commands::set_theme,
            scan::commands::pick_scan_roots,
            scan::commands::scan_snapshot,
            scan::commands::update_scan_root,
            scan::commands::start_scan,
            scan::commands::control_scan,
            scan::commands::list_imported_books,
            metadata::commands::next_cover_task,
            metadata::commands::read_cover_range,
            metadata::commands::finish_cover,
            metadata::commands::get_book_cover,
            metadata::commands::metadata_review_page,
            metadata::commands::save_book_metadata,
            metadata::commands::list_duplicate_groups,
            metadata::commands::set_duplicate_review,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start LibraDesk");
}
