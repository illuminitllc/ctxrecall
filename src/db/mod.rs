pub mod config_repo;
pub mod document_repo;
pub mod issue_repo;
pub mod search_repo;
pub mod session_repo;

use std::path::Path;

use rusqlite::Connection;

use crate::errors::AppError;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial",
        include_str!("../../migrations/001_initial.sql"),
    ),
    (
        "002_issues",
        include_str!("../../migrations/002_issues.sql"),
    ),
    (
        "003_transcripts",
        include_str!("../../migrations/003_transcripts.sql"),
    ),
    (
        "004_fts",
        include_str!("../../migrations/004_fts.sql"),
    ),
    (
        "005_issue_status_id",
        include_str!("../../migrations/005_issue_status_id.sql"),
    ),
    (
        "006_workflow_states",
        include_str!("../../migrations/006_workflow_states.sql"),
    ),
    (
        "007_issue_team_id",
        include_str!("../../migrations/007_issue_team_id.sql"),
    ),
    (
        "008_issue_project_id",
        include_str!("../../migrations/008_issue_project_id.sql"),
    ),
    (
        "009_labels",
        include_str!("../../migrations/009_labels.sql"),
    ),
    (
        "010_issue_assignee_id",
        include_str!("../../migrations/010_issue_assignee_id.sql"),
    ),
    (
        "011_label_team_id",
        include_str!("../../migrations/011_label_team_id.sql"),
    ),
    (
        "012_document_file_path",
        include_str!("../../migrations/012_document_file_path.sql"),
    ),
    (
        "013_status_hotkeys",
        include_str!("../../migrations/013_status_hotkeys.sql"),
    ),
    (
        "014_seed_themes",
        include_str!("../../migrations/014_seed_themes.sql"),
    ),
    (
        "015_add_nord_last_horizon_themes",
        include_str!("../../migrations/015_add_nord_last_horizon_themes.sql"),
    ),
];

pub fn init_db(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )?;

    run_migrations(&conn)?;

    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    for (name, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM schema_migrations WHERE name = ?1",
            [name],
            |row| row.get(0),
        )?;

        if !applied {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (name) VALUES (?1)",
                [name],
            )?;
            tracing::info!("Applied migration: {name}");
        }
    }

    Ok(())
}
