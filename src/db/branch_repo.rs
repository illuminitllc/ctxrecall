use rusqlite::Connection;

use crate::errors::AppError;

pub fn set_branch(conn: &Connection, issue_id: &str, branch_name: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO issue_branches (issue_id, branch_name, created_at) VALUES (?1, ?2, datetime('now'))",
        [issue_id, branch_name],
    )?;
    Ok(())
}

pub fn get_branch(conn: &Connection, issue_id: &str) -> Result<Option<String>, AppError> {
    let mut stmt = conn.prepare("SELECT branch_name FROM issue_branches WHERE issue_id = ?1")?;
    let result = stmt
        .query_row([issue_id], |row| row.get::<_, String>(0))
        .ok();
    Ok(result)
}

pub fn clear_branch(conn: &Connection, issue_id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM issue_branches WHERE issue_id = ?1", [issue_id])?;
    Ok(())
}
