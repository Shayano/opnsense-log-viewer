// Modules
mod commands;
pub mod parser; // Parser module - Public for integration tests
pub mod types;
pub mod indexer;
pub mod storage;
pub mod query;
mod api_client; // Story 3.1: API client and connection setup
mod credentials; // Story 3.1: Credential storage
mod state; // Story 3.2: Application state management

// Re-export types for use in other modules
pub use types::{FileMetadata, IndexMetadata};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize enrichment cache state
    let enrichment_cache = state::EnrichmentCacheState::new();

    tauri::Builder::default()
        .manage(enrichment_cache) // Register enrichment cache state
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
            api_client::commands::test_api_connection,
            // Story 3.2: Interface Mapping & Enrichment
            api_client::commands::fetch_interface_mappings_cmd,
            api_client::commands::get_interface_mappings_cmd,
            api_client::commands::get_logical_interface_name,
            // Story 3.3: Rule Label Enrichment
            api_client::commands::fetch_rule_labels,
            api_client::commands::get_rule_labels,
            api_client::commands::get_rule_label,
            // Story 3.4: Alias Resolution
            api_client::commands::fetch_aliases,
            api_client::commands::get_aliases,
            api_client::commands::get_alias
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
