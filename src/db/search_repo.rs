use rusqlite::Connection;

use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub source_type: String,
    pub source_id: String,
    pub issue_id: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

pub fn index_content(
    conn: &Connection,
    source_type: &str,
    source_id: &str,
    issue_id: &str,
    title: &str,
    content: &str,
) -> Result<(), AppError> {
    // Remove existing entry first
    conn.execute(
        "DELETE FROM search_index WHERE source_type = ?1 AND source_id = ?2",
        rusqlite::params![source_type, source_id],
    )?;

    conn.execute(
        "INSERT INTO search_index (source_type, source_id, issue_id, title, content)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![source_type, source_id, issue_id, title, content],
    )?;

    Ok(())
}

pub fn search(
    conn: &Connection,
    query: &str,
    scope_issue_id: Option<&str>,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let fts_query = format!("{query}*"); // Prefix matching

    let results = if let Some(issue_id) = scope_issue_id {
        let mut stmt = conn.prepare(
            "SELECT source_type, source_id, issue_id, title,
                    snippet(search_index, 4, '<b>', '</b>', '...', 32) as snippet,
                    rank
             FROM search_index
             WHERE search_index MATCH ?1 AND issue_id = ?2
             ORDER BY rank
             LIMIT ?3",
        )?;

        stmt.query_map(rusqlite::params![fts_query, issue_id, limit as i64], |row| {
            Ok(SearchResult {
                source_type: row.get(0)?,
                source_id: row.get(1)?,
                issue_id: row.get(2)?,
                title: row.get(3)?,
                snippet: row.get(4)?,
                rank: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT source_type, source_id, issue_id, title,
                    snippet(search_index, 4, '<b>', '</b>', '...', 32) as snippet,
                    rank
             FROM search_index
             WHERE search_index MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        stmt.query_map(rusqlite::params![fts_query, limit as i64], |row| {
            Ok(SearchResult {
                source_type: row.get(0)?,
                source_id: row.get(1)?,
                issue_id: row.get(2)?,
                title: row.get(3)?,
                snippet: row.get(4)?,
                rank: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
    };

    Ok(results)
}
