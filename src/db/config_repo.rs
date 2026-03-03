use rusqlite::Connection;
use uuid::Uuid;

use crate::config::theme::Theme;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct AccountRow {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api_key: String,
    pub is_active: bool,
    pub model: Option<String>,
    pub ollama_url: Option<String>,
}

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

pub fn list_config_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut stmt = conn.prepare("SELECT key, value FROM config WHERE key LIKE ?1")?;
    let pattern = format!("{prefix}%");
    let rows = stmt.query_map([&pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn delete_config(conn: &Connection, key: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM config WHERE key = ?1", [key])?;
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

pub fn list_accounts(conn: &Connection, providers: &[&str]) -> Result<Vec<AccountRow>, AppError> {
    if providers.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = providers.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT a.id, a.name, a.provider, a.api_key, a.is_active, cm.value, cu.value \
         FROM accounts a \
         LEFT JOIN config cm ON cm.key = 'llm_model:' || a.id \
         LEFT JOIN config cu ON cu.key = 'llm_ollama_url:' || a.id \
         WHERE a.provider IN ({}) ORDER BY a.name",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = providers.iter().map(|p| p as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok(AccountRow {
            id: row.get(0)?,
            name: row.get(1)?,
            provider: row.get(2)?,
            api_key: row.get(3)?,
            is_active: row.get::<_, i32>(4)? != 0,
            model: row.get(5)?,
            ollama_url: row.get(6)?,
        })
    })?;
    let mut accounts = Vec::new();
    for row in rows {
        accounts.push(row?);
    }
    Ok(accounts)
}

pub fn get_account(conn: &Connection, id: &str) -> Result<Option<AccountRow>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.name, a.provider, a.api_key, a.is_active, cm.value, cu.value \
         FROM accounts a \
         LEFT JOIN config cm ON cm.key = 'llm_model:' || a.id \
         LEFT JOIN config cu ON cu.key = 'llm_ollama_url:' || a.id \
         WHERE a.id = ?1",
    )?;
    let result = stmt
        .query_row([id], |row| {
            Ok(AccountRow {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                api_key: row.get(3)?,
                is_active: row.get::<_, i32>(4)? != 0,
                model: row.get(5)?,
                ollama_url: row.get(6)?,
            })
        })
        .ok();
    Ok(result)
}

pub fn insert_account(conn: &Connection, name: &str, provider: &str, api_key: &str) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO accounts (id, name, provider, api_key, is_active, created_at) VALUES (?1, ?2, ?3, ?4, 0, datetime('now'))",
        rusqlite::params![id, name, provider, api_key],
    )?;
    Ok(id)
}

pub fn update_account(conn: &Connection, id: &str, name: &str, api_key: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE accounts SET name = ?2, api_key = ?3 WHERE id = ?1",
        rusqlite::params![id, name, api_key],
    )?;
    Ok(())
}

pub fn delete_account(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
    // Clean up any LLM config keys for this account
    conn.execute("DELETE FROM config WHERE key = ?1", [&format!("llm_model:{id}")])?;
    conn.execute("DELETE FROM config WHERE key = ?1", [&format!("llm_ollama_url:{id}")])?;
    Ok(())
}

pub fn set_active_account(conn: &Connection, id: &str, provider_group: &[&str]) -> Result<(), AppError> {
    if provider_group.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = provider_group.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let deactivate_sql = format!(
        "UPDATE accounts SET is_active = 0 WHERE provider IN ({})",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&deactivate_sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = provider_group.iter().map(|p| p as &dyn rusqlite::types::ToSql).collect();
    stmt.execute(params.as_slice())?;
    conn.execute("UPDATE accounts SET is_active = 1 WHERE id = ?1", [id])?;
    Ok(())
}

pub fn set_account_llm_config(conn: &Connection, account_id: &str, model: Option<&str>, ollama_url: Option<&str>) -> Result<(), AppError> {
    if let Some(m) = model {
        set_config(conn, &format!("llm_model:{account_id}"), m)?;
    } else {
        conn.execute("DELETE FROM config WHERE key = ?1", [&format!("llm_model:{account_id}")])?;
    }
    if let Some(u) = ollama_url {
        set_config(conn, &format!("llm_ollama_url:{account_id}"), u)?;
    } else {
        conn.execute("DELETE FROM config WHERE key = ?1", [&format!("llm_ollama_url:{account_id}")])?;
    }
    Ok(())
}

pub fn get_account_llm_config(conn: &Connection, account_id: &str) -> (Option<String>, Option<String>) {
    let model = get_config(conn, &format!("llm_model:{account_id}")).ok().flatten();
    let ollama_url = get_config(conn, &format!("llm_ollama_url:{account_id}")).ok().flatten();
    (model, ollama_url)
}

// ── Theme persistence ──

pub fn get_active_theme(conn: &Connection) -> Option<Theme> {
    let mut stmt = conn
        .prepare("SELECT data FROM themes WHERE is_active = 1 LIMIT 1")
        .ok()?;
    let json: String = stmt.query_row([], |row| row.get(0)).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn set_active_theme(conn: &Connection, name: &str, theme: &Theme) -> Result<(), AppError> {
    let json = serde_json::to_string(theme)
        .map_err(|e| AppError::Config(format!("Failed to serialize theme: {e}")))?;
    conn.execute("UPDATE themes SET is_active = 0", [])?;
    conn.execute(
        "INSERT INTO themes (name, data, is_active) VALUES (?1, ?2, 1)
         ON CONFLICT(name) DO UPDATE SET data = ?2, is_active = 1",
        rusqlite::params![name, json],
    )?;
    Ok(())
}
