use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::time::{sleep, Duration};
use tauri::Emitter;

pub fn start_scheduler(db_path: PathBuf, app_handle: tauri::AppHandle) {
    tokio::spawn(async move {
        loop {
            // Run checks every hour
            if let Ok(conn) = Connection::open(&db_path) {
                let _ = run_due_automations(&conn, &db_path, &app_handle).await;
            }
            // Sleep for 1 hour (3600 seconds)
            sleep(Duration::from_secs(3600)).await;
        }
    });
}

async fn run_due_automations(conn: &Connection, db_path: &Path, app_handle: &tauri::AppHandle) -> Result<(), String> {
    // 1. Database Backup automation check
    if is_automation_due(conn, "Database Backup", 1)? {
        if let Ok(backup_path) = get_setting(conn, "backup_path") {
            if !backup_path.is_empty() {
                let backup_dir = Path::new(&backup_path);
                if backup_dir.exists() {
                    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                    let dest = backup_dir.join(format!("collection_ho_backup_{}.db", timestamp));
                    fs::copy(db_path, dest).map_err(|e| e.to_string())?;
                    update_automation_last_run(conn, "Database Backup")?;
                    let _ = app_handle.emit("automation-run", "Database Backup Successful");
                }
            }
        }
    }

    // 2. Weekly Performance Report check
    if is_automation_due(conn, "Weekly Performance Report", 7)? {
        if let Ok(backup_path) = get_setting(conn, "backup_path") {
            if !backup_path.is_empty() {
                let backup_dir = Path::new(&backup_path);
                if backup_dir.exists() {
                    let timestamp = chrono::Utc::now().format("%Y%m%d").to_string();
                    let dest = backup_dir.join(format!("weekly_report_{}.txt", timestamp));
                    
                    // Generate a quick text report
                    if let Ok(report_text) = compile_weekly_summary(conn) {
                        fs::write(dest, report_text).map_err(|e| e.to_string())?;
                        update_automation_last_run(conn, "Weekly Performance Report")?;
                        let _ = app_handle.emit("automation-run", "Weekly Performance Report Created");
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_setting(conn: &Connection, key: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    ).map_err(|e| e.to_string())
}

fn is_automation_due(conn: &Connection, name: &str, interval_days: i64) -> Result<bool, String> {
    let row: Option<(String, i64)> = conn.query_row(
        "SELECT last_run, active FROM automations WHERE name = ?1",
        [name],
        |row| {
            let last_run: Option<String> = row.get(0)?;
            let active: i64 = row.get(1)?;
            Ok((last_run.unwrap_or_default(), active))
        },
    ).ok();

    if let Some((last_run, active)) = row {
        if active == 0 {
            return Ok(false);
        }
        if last_run.is_empty() {
            return Ok(true);
        }
        if let Ok(last_run_time) = chrono::DateTime::parse_from_rfc3339(&last_run) {
            let duration = chrono::Utc::now().signed_duration_since(last_run_time.with_timezone(&chrono::Utc));
            return Ok(duration.num_days() >= interval_days);
        }
    }
    Ok(false)
}

fn update_automation_last_run(conn: &Connection, name: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE automations SET last_run = ?1 WHERE name = ?2",
        params![now, name],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn compile_weekly_summary(conn: &Connection) -> Result<String, String> {
    let last_week = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
    
    let (total_orders, sales, profit): (i64, f64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(total_amount), 0.0), COALESCE(SUM(profit), 0.0) 
         FROM orders WHERE order_date >= ?1",
        [&last_week],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| e.to_string())?;

    let low_stock_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products WHERE stock_quantity <= 5 AND status = 'active'",
        [],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let report = format!(
        "=========================================\n\
         WEEKLY BUSINESS SUMMARY REPORT\n\
         Date: {}\n\
         =========================================\n\n\
         Sales Activity (Last 7 Days):\n\
         - Total Orders: {}\n\
         - Gross Sales: ${:.2}\n\
         - Total Profit: ${:.2}\n\n\
         Inventory Health:\n\
         - Low Stock Items: {}\n\n\
         Generated automatically by A Collection Head Office Operating System.\n\
         =========================================",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        total_orders,
        sales,
        profit,
        low_stock_count
    );

    Ok(report)
}
