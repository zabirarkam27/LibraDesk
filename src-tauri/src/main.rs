#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn check_connection() -> &'static str {
    "Desktop connection is working."
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![check_connection])
        .run(tauri::generate_context!())
        .expect("failed to start LibraDesk");
}
