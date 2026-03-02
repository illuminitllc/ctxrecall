use rusqlite::Connection;

use crate::errors::AppError;

#[derive(Debug, Clone, Default)]
pub struct SessionDashboardStats {
    pub total_sessions: usize,
    pub active_session_issue: Option<String>,
    pub last_session_issue: Option<String>,
    pub last_session_time: Option<String>,
}

pub fn get_dashboard_session_stats(conn: &Connection) -> Result<SessionDashboardStats, AppError> {
    let total_sessions: usize = conn.query_row(
        "SELECT COUNT(*) FROM claude_sessions",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let active_session_issue: Option<String> = conn
        .query_row(
            "SELECT i.identifier FROM claude_sessions s
             LEFT JOIN issues i ON i.id = s.issue_id
             WHERE s.ended_at IS NULL
             ORDER BY s.started_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    let last_session: Option<(String, String)> = conn
        .query_row(
            "SELECT COALESCE(i.identifier, s.issue_id), s.started_at
             FROM claude_sessions s
             LEFT JOIN issues i ON i.id = s.issue_id
             WHERE s.ended_at IS NOT NULL
             ORDER BY s.started_at DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    let (last_session_issue, last_session_time) = match last_session {
        Some((issue, time)) => (Some(issue), Some(time)),
        None => (None, None),
    };

    Ok(SessionDashboardStats {
        total_sessions,
        active_session_issue,
        last_session_issue,
        last_session_time,
    })
}

pub fn save_session(
    conn: &Connection,
    id: &str,
    issue_id: &str,
    session_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO claude_sessions (id, issue_id, session_id, started_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET session_id = ?3",
        rusqlite::params![id, issue_id, session_id],
    )?;
    Ok(())
}

pub fn end_session(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE claude_sessions SET ended_at = datetime('now') WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

pub fn get_active_session_for_issue(
    conn: &Connection,
    issue_id: &str,
) -> Result<Option<(String, String)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id FROM claude_sessions
         WHERE issue_id = ?1 AND ended_at IS NULL
         ORDER BY started_at DESC LIMIT 1",
    )?;

    let result = stmt
        .query_row([issue_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .ok();

    Ok(result)
}

pub fn get_latest_session_id_for_issue(
    conn: &Connection,
    issue_id: &str,
) -> Result<Option<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT session_id FROM claude_sessions
         WHERE issue_id = ?1
         ORDER BY started_at DESC LIMIT 1",
    )?;

    let result = stmt.query_row([issue_id], |row| row.get::<_, String>(0)).ok();

    Ok(result)
}
