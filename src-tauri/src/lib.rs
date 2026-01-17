// Modules
mod commands;
pub mod parser; // Parser module - Public for integration tests
pub mod types;
pub mod indexer;

// Re-export types for use in other modules
pub use types::{FileMetadata, IndexMetadata};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::indexation::get_file_metadata,
            commands::indexation::index_file,
            commands::indexation::index_file_with_format,
            commands::indexation::build_hybrid_index,
            commands::indexation::cancel_indexation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
