use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    pub id: Option<i64>,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

pub fn get_all_locations(conn: &Connection) -> Result<Vec<Location>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, name, address, is_active, created_at FROM locations ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Location {
            id: Some(row.get(0)?), name: row.get(1)?, address: row.get(2)?,
            is_active: row.get::<_, i64>(3)? != 0, created_at: row.get(4)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows { result.push(r?); }
    Ok(result)
}

pub fn add_location(conn: &Connection, name: &str, address: &str) -> Result<i64, rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("INSERT INTO locations (name, address, created_at) VALUES (?1, ?2, ?3)", params![name, address, now])?;
    Ok(conn.last_insert_rowid())
}

pub fn update_location(conn: &Connection, id: i64, name: &str, address: &str, is_active: bool) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE locations SET name=?1, address=?2, is_active=?3 WHERE id=?4",
        params![name, address, is_active as i64, id],
    )?;
    Ok(())
}
