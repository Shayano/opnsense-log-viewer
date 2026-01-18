// Modules
mod commands;
pub mod parser; // Parser module - Public for integration tests
pub mod types;
pub mod indexer;
pub mod storage;
pub mod query;
mod api_client; // Story 3.1: API client and connection setup
mod credentials; // Story 3.1: Credential storage

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
            commands::indexation::cancel_indexation,
            commands::storage::list_all_indexes,
            commands::storage::delete_index_by_hash,
            commands::storage::load_index_file,
            commands::query::execute_query,
            // Story 3.1: API Connection & Credential Storage
            api_client::commands::save_api_credentials,
            api_client::commands::load_api_credentials,
            api_client::commands::test_api_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
