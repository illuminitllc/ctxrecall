use std::path::{Path, PathBuf};

use rusqlite::Connection;
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub issue_id: String,
    pub doc_type: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub file_path: Option<String>,
}

pub fn create_document(
    conn: &Connection,
    issue_id: &str,
    doc_type: &str,
    title: &str,
    content: &str,
    file_path: Option<&str>,
) -> Result<Document, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO documents (id, issue_id, doc_type, title, content, file_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, issue_id, doc_type, title, content, file_path],
    )?;

    get_document(conn, &id)
}

pub fn update_document(
    conn: &Connection,
    id: &str,
    title: &str,
    content: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE documents SET title = ?2, content = ?3, updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id, title, content],
    )?;
    Ok(())
}

pub fn get_document(conn: &Connection, id: &str) -> Result<Document, AppError> {
    conn.query_row(
        "SELECT id, issue_id, doc_type, title, content, created_at, updated_at, file_path
         FROM documents WHERE id = ?1",
        [id],
        |row| {
            Ok(Document {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                doc_type: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                file_path: row.get(7)?,
            })
        },
    )
    .map_err(AppError::Database)
}

pub fn list_documents_for_issue(
    conn: &Connection,
    issue_id: &str,
) -> Result<Vec<Document>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, issue_id, doc_type, title, content, created_at, updated_at, file_path
         FROM documents WHERE issue_id = ?1 ORDER BY doc_type, updated_at DESC",
    )?;

    let docs = stmt
        .query_map([issue_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                doc_type: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                file_path: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(docs)
}

pub fn delete_document(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM documents WHERE id = ?1", [id])?;
    Ok(())
}

// --- File utilities ---

/// Generate a filename like `ENG-123_plan_my-doc-title.md`
pub fn doc_filename(identifier: &str, doc_type: &str, title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.len() > 50 { &slug[..50] } else { &slug };
    format!("{identifier}_{doc_type}_{slug}.md")
}

/// Resolve the docs directory: project dir preferred, data dir fallback.
pub fn resolve_docs_dir(
    working_dir: Option<&str>,
    data_dir: &Path,
    issue_id: &str,
) -> PathBuf {
    if let Some(dir) = working_dir {
        PathBuf::from(dir).join("docs")
    } else {
        data_dir.join("docs").join(issue_id)
    }
}

/// Write a markdown document file, creating directories as needed.
pub fn write_doc_file(
    docs_dir: &Path,
    filename: &str,
    title: &str,
    doc_type: &str,
    content: &str,
) -> Result<PathBuf, AppError> {
    std::fs::create_dir_all(docs_dir)?;
    let path = docs_dir.join(filename);
    let body = if content.is_empty() {
        format!("# {title}\n\nType: {doc_type}\n")
    } else {
        content.to_string()
    };
    std::fs::write(&path, &body)?;
    Ok(path)
}
