#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod catalog;
mod catalog_publish;
mod inventory;
mod customers;
mod reports;
mod locations;
mod agents;
mod purchase_trips;
mod adapters;
mod ai;
mod automation;
mod utils;
mod commands;

use commands::DbState;

fn main() {
    let db_path = utils::get_db_path();
    let conn = database::init_db(&db_path).expect("Failed to initialize SQLite database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // v0.14.5: Clipboard manager for image sharing. Lets the frontend
        // call writeImage(bytes) to put a product image on the system
        // clipboard, so the user can paste it into FB/IG/WhatsApp post
        // composers after we open the share URL.
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(DbState(tauri::async_runtime::Mutex::new(conn)))
        .setup(move |app| {
            let app_handle = app.handle().clone();
            automation::start_scheduler(db_path, app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_products,
            commands::get_product,
            commands::add_product,
            commands::update_product,
            commands::delete_product,
            commands::get_product_locations,
            commands::upsert_product_location,
            commands::search_products_by_color,
            commands::export_products_csv,
            commands::import_products_csv,
            commands::upload_product_image,
            commands::get_image_as_base64,
            commands::save_base64_image,
            commands::save_image_for_share,
            commands::save_drafts_to_folder_with_path,
            commands::create_full_backup,
            commands::list_backups,
            commands::restore_backup,
            commands::import_from_catalog_json,
            commands::get_inventory_summary,
            commands::get_low_stock,
            commands::get_dead_stock,
            commands::get_best_sellers,
            commands::adjust_stock,
            commands::get_customers,
            commands::add_customer,
            commands::update_customer,
            commands::delete_customer,
            commands::create_order,
            commands::get_customer_history,
            commands::get_sales_report,
            commands::get_inventory_report,
            commands::get_customer_report,
            commands::ask_ai,
            commands::get_settings,
            commands::update_setting,
            commands::backup_database_now,
            commands::get_knowledge,
            commands::save_knowledge,
            commands::delete_knowledge,
            commands::get_locations,
            commands::add_location,
            commands::update_location,
            commands::save_product_draft_to_catalog,
            commands::save_catalog_draft,
            commands::generate_social_post,
            commands::generate_marketing,
            commands::init_database,
            // v0.11.0 — Agents
            commands::get_agents,
            commands::get_agent,
            commands::add_agent,
            commands::update_agent,
            commands::delete_agent,
            commands::get_agent_ledger,
            commands::update_agent_ledger_entry,
            commands::delete_agent_ledger_entry,
            commands::get_product_agent_stock,
            commands::send_stock_to_agent,
            commands::return_stock_from_agent,
            commands::report_agent_sale,
            commands::receive_agent_cash,
            commands::adjust_agent_balance,
            // v0.11.1 — Share Center
            commands::log_share,
            commands::get_share_logs,
            commands::get_customers_by_segment,
            commands::update_customer_segment,
            commands::get_customer_segments,
            commands::get_stale_products,
            // v0.11.2 — Purchase Trips
            commands::get_purchase_trips,
            commands::get_purchase_trip,
            commands::create_purchase_trip,
            commands::update_purchase_trip,
            commands::delete_purchase_trip,
            commands::add_trip_item,
            commands::remove_trip_item,
            commands::recalculate_trip,
            // v0.12.5 — Sales
            commands::record_sale,
            commands::get_sales,
            // v0.15.0 — Public Catalog Publishing
            commands::preview_catalog_publish,
            commands::publish_catalog_to_github,
            commands::get_catalog_publish_history,
            // v0.23.0 — Page-Agent Integration (Phase 1: Foundation)
            commands::page_agent_invoke,
            commands::page_agent_get_context,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
