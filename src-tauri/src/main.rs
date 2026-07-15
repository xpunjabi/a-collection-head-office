#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod catalog;
mod catalog_publish;
mod inventory;
mod customers;
mod reports;
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
            commands::products_commands::get_products,
            commands::products_commands::add_product,
            commands::products_commands::update_product,
            commands::products_commands::delete_product,
            commands::products_commands::export_products_csv,
            commands::products_commands::import_products_csv,
            commands::products_commands::upload_product_image,
            commands::products_commands::get_image_as_base64,
            commands::products_commands::save_base64_image,
            commands::products_commands::save_image_from_url,
            commands::products_commands::save_image_for_share,
            commands::products_commands::save_drafts_to_folder_with_path,
            // v0.33.0 — Manual sold-out marking
            commands::products_commands::mark_product_sold_out,
            commands::backup_commands::list_backups,
            commands::backup_commands::restore_backup,
            commands::backup_commands::import_from_catalog_json,
            commands::inventory_commands::get_inventory_summary,
            commands::inventory_commands::get_low_stock,
            commands::inventory_commands::get_dead_stock,
            commands::inventory_commands::get_best_sellers,
            commands::inventory_commands::adjust_stock,
            commands::customers_commands::get_customers,
            commands::customers_commands::add_customer,
            commands::customers_commands::update_customer,
            commands::customers_commands::delete_customer,
            commands::customers_commands::create_order,
            commands::customers_commands::get_customer_history,
            commands::reports_commands::get_sales_report,
            commands::reports_commands::get_inventory_report,
            commands::reports_commands::get_customer_report,
            commands::ai_commands::ask_ai,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_setting,
            commands::backup_commands::backup_database_now,
            commands::ai_commands::save_knowledge,
            commands::ai_commands::save_product_draft_to_catalog,
            commands::ai_commands::save_catalog_draft,
            commands::ai_commands::generate_social_post,
            commands::ai_commands::generate_marketing,
            commands::backup_commands::init_database,
            // v0.11.0 — Agents
            commands::agents_commands::get_agents,
            commands::agents_commands::add_agent,
            commands::agents_commands::update_agent,
            commands::agents_commands::delete_agent,
            commands::agents_commands::get_agent_ledger,
            commands::agents_commands::get_product_agent_stock,
            commands::agents_commands::send_stock_to_agent,
            commands::agents_commands::return_stock_from_agent,
            commands::agents_commands::report_agent_sale,
            commands::agents_commands::receive_agent_cash,
            commands::agents_commands::adjust_agent_balance,
            // v0.29.0 — Agent manual ledger entries (maal value + advance + edit/delete)
            commands::agents_commands::add_agent_manual_entry,
            commands::agents_commands::update_agent_ledger_entry,
            commands::agents_commands::delete_agent_ledger_entry,
            // v0.11.1 — Share Center
            commands::share_segments_commands::log_share,
            commands::share_segments_commands::get_share_logs,
            commands::share_segments_commands::get_customers_by_segment,
            commands::share_segments_commands::update_customer_segment,
            commands::share_segments_commands::get_customer_segments,
            commands::share_segments_commands::get_stale_products,
            // v0.11.2 — Purchase Trips
            commands::purchase_trips_commands::get_purchase_trips,
            commands::purchase_trips_commands::get_purchase_trip,
            commands::purchase_trips_commands::create_purchase_trip,
            commands::purchase_trips_commands::update_purchase_trip,
            commands::purchase_trips_commands::delete_purchase_trip,
            commands::purchase_trips_commands::add_trip_item,
            commands::purchase_trips_commands::remove_trip_item,
            // v0.12.5 — Sales
            commands::sales_commands::record_sale,
            // v0.30.0 — Sale undo + sold items reactivation
            commands::sales_commands::undo_sale,
            commands::sales_commands::reactivate_sold_product,
            // v0.26.0 — Customer Udhar/Credit (खाता)
            commands::udhar_commands::record_customer_payment,
            commands::udhar_commands::get_customer_balance_history,
            // v0.29.0 — Customer manual ledger entries (opening balance + adjustments)
            commands::udhar_commands::add_customer_ledger_entry,
            commands::udhar_commands::update_customer_ledger_entry,
            commands::udhar_commands::delete_customer_ledger_entry,
            // v0.15.0 — Public Catalog Publishing
            commands::catalog_publish_commands::preview_catalog_publish,
            commands::catalog_publish_commands::publish_catalog_to_github,
            commands::catalog_publish_commands::get_catalog_publish_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
