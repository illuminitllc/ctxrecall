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
}

pub fn create_document(
    conn: &Connection,
    issue_id: &str,
    doc_type: &str,
    title: &str,
    content: &str,
) -> Result<Document, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO documents (id, issue_id, doc_type, title, content)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, issue_id, doc_type, title, content],
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
        "SELECT id, issue_id, doc_type, title, content, created_at, updated_at
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
        "SELECT id, issue_id, doc_type, title, content, created_at, updated_at
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
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(docs)
}

pub fn delete_document(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM documents WHERE id = ?1", [id])?;
    Ok(())
}
