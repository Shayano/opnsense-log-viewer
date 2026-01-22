// Story 6.1: Use mimalloc as global allocator to reduce allocation contention
// on multi-threaded indexing (reduces lock contention on default allocator)
// Note: Conditionally enabled to avoid conflicts with test allocators (e.g., PeakAlloc)
#[cfg(feature = "mimalloc-allocator")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
mod export; // Story 5.1: Export functionality

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
    let cache_clone = enrichment_cache.clone();

    tauri::Builder::default()
        .manage(enrichment_cache) // Register enrichment cache state
        .setup(move |_app| {
            // Initialize logging - logs visible with RUST_LOG=debug or RUST_LOG=info
            tracing_subscriber::fmt::init();
            // log::info! / [MEM] logs need env_logger; use RUST_LOG=info to see them
            let _ = env_logger::try_init();

            // Spawn background health check task (Story 3.5)
            // Must be inside setup() because Tokio runtime is only available after Tauri starts
            tauri::async_runtime::spawn(async move {
                use tokio::time::{interval, Duration};
                use log::{debug, info};

                let mut interval_timer = interval(Duration::from_secs(30));

                loop {
                    interval_timer.tick().await;

                    // Clean up caches to prevent memory leaks (every 30 seconds)
                    // Max age: 1 hour, Max rule labels: 10000, Max aliases: 5000
                    cache_clone.cleanup_cache(3600, 10000, 5000);

                    // Only attempt reconnect if currently disconnected
                    if !cache_clone.is_connected() {
                        debug!("Background health check: attempting reconnect");

                        // Load credentials
                        if let Some(credentials) = credentials::manager::load_credentials()
                            .ok()
                            .flatten()
                            .or_else(|| credentials::encrypted_storage::decrypt_and_load().ok().flatten())
                        {
                            // Attempt silent reconnection
                            match api_client::client::test_connection(&credentials).await {
                                Ok(_) => {
                                    info!("Background reconnection succeeded");
                                    cache_clone.set_connection_status(
                                        api_client::types::ConnectionStatus::Connected,
                                        None
                                    );
                                }
                                Err(e) => {
                                    debug!("Background reconnection failed: {}", e);
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::indexation::get_file_metadata,
            commands::indexation::index_file,
            commands::indexation::index_file_with_format,
            commands::indexation::build_hybrid_index,
            commands::indexation::cancel_indexation,
            // Story 6.3: SQLite-based indexation with real-time progress
            commands::sqlite_indexation::build_sqlite_index,
            commands::sqlite_indexation::cancel_sqlite_indexation,
            commands::storage::list_all_indexes,
            commands::storage::delete_index_by_hash,
            commands::storage::check_index_exists, // Story 1.7: Quick index existence check
            commands::storage::load_index_file,
            commands::query::execute_query,
            commands::query::get_entries_by_ids,
            commands::query::get_bitmap_stats,
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
            api_client::commands::get_alias,
            // Story 3.5: Connection Management
            api_client::commands::get_connection_status,
            api_client::commands::retry_api_connection,
            // Story 4.1: Enrichment Export
            api_client::commands::export_enrichment_data,
            api_client::commands::save_enrichment_export,
            api_client::commands::open_folder,
            // Story 4.2: Enrichment Import
            api_client::commands::validate_enrichment_import,
            api_client::commands::import_enrichment_data,
            api_client::commands::open_enrichment_file_picker,
            // Story 4.3: Staleness Indicators
            api_client::commands::reconnect_api,
            api_client::commands::clear_backup_enrichment,
            api_client::commands::set_staleness_indicator_dismissed,
            api_client::commands::get_staleness_indicator_dismissed,
            // Story 5.1: Export Functionality
            export::commands::export_filtered_results,
            export::commands::cancel_export,
            export::commands::open_export_location,
            // Story 5.2: Full Dataset Export with Streaming
            export::commands::estimate_export,
            // Story 5.3: Export Integrity & Verification
            export::commands::verify_export_file,
            export::commands::detect_incomplete_export_files,
            export::commands::cleanup_incomplete_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
