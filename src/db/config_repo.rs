use rusqlite::Connection;

use crate::errors::AppError;

pub fn get_config(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;
    let result = stmt
        .query_row([key], |row| row.get::<_, String>(0))
        .ok();
    Ok(result)
}

pub fn set_config(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO config (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
        [key, value],
    )?;
    Ok(())
}

pub fn get_active_api_key(conn: &Connection) -> Result<Option<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT api_key FROM accounts WHERE is_active = 1 AND provider = 'linear' LIMIT 1",
    )?;
    let result = stmt
        .query_row([], |row| row.get::<_, String>(0))
        .ok();
    Ok(result)
}
