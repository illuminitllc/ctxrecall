use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::theme::Theme;
use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub hotkeys: HashMap<String, String>,
    #[serde(default)]
    pub theme: Option<Theme>,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub sync_interval_secs: Option<u64>,
    pub transcript_capture_interval_secs: Option<u64>,
    pub summarize_interval_mins: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountConfig {
    pub name: String,
    pub provider: String,
    pub api_key: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub ollama_url: Option<String>,
}

pub fn import_config(conn: &Connection, path: &Path) -> Result<AppConfig, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::Config(format!("Failed to read config: {e}")))?;

    let config: AppConfig = toml::from_str(&content)
        .map_err(|e| AppError::Config(format!("Failed to parse TOML: {e}")))?;

    // Import hotkeys
    for (action, binding) in &config.hotkeys {
        conn.execute(
            "UPDATE hotkeys SET key_binding = ?2, updated_at = datetime('now') WHERE action = ?1",
            rusqlite::params![action, binding],
        )?;
    }

    // Import accounts
    for account in &config.accounts {
        conn.execute(
            "INSERT INTO accounts (id, name, provider, api_key, is_active)
             VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)
             ON CONFLICT DO NOTHING",
            rusqlite::params![
                account.name,
                account.provider,
                account.api_key,
                account.active as i32
            ],
        )?;
    }

    // Import general settings
    if let Some(interval) = config.general.sync_interval_secs {
        crate::db::config_repo::set_config(conn, "sync_interval_secs", &interval.to_string())?;
    }

    // Import LLM settings
    if let Some(provider) = &config.llm.provider {
        crate::db::config_repo::set_config(conn, "llm_provider", provider)?;
    }
    if let Some(key) = &config.llm.api_key {
        crate::db::config_repo::set_config(conn, "llm_api_key", key)?;
    }
    if let Some(model) = &config.llm.model {
        crate::db::config_repo::set_config(conn, "llm_model", model)?;
    }

    // Import theme
    if let Some(ref theme) = config.theme {
        crate::db::config_repo::set_active_theme(conn, &theme.name, theme)?;
    }

    tracing::info!("Config imported from {}", path.display());
    Ok(config)
}

pub fn export_config(conn: &Connection, path: &Path) -> Result<(), AppError> {
    let mut hotkeys = HashMap::new();
    let mut stmt = conn.prepare("SELECT action, key_binding FROM hotkeys")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows.flatten() {
        hotkeys.insert(row.0, row.1);
    }

    let mut accounts = Vec::new();
    let mut stmt = conn.prepare("SELECT name, provider, api_key, is_active FROM accounts")?;
    let rows = stmt.query_map([], |row| {
        Ok(AccountConfig {
            name: row.get(0)?,
            provider: row.get(1)?,
            api_key: row.get(2)?,
            active: row.get::<_, i32>(3)? != 0,
        })
    })?;
    for row in rows.flatten() {
        accounts.push(row);
    }

    let theme = crate::db::config_repo::get_active_theme(conn);

    let config = AppConfig {
        general: GeneralConfig::default(),
        hotkeys,
        theme,
        accounts,
        llm: LlmConfig::default(),
    };

    let content = toml::to_string_pretty(&config)
        .map_err(|e| AppError::Config(format!("Failed to serialize: {e}")))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;

    tracing::info!("Config exported to {}", path.display());
    Ok(())
}
