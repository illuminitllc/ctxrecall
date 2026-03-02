use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use rusqlite::Connection;

use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct HotkeyBinding {
    pub action: String,
    pub key_binding: String,
    pub description: Option<String>,
}

pub fn load_hotkeys(conn: &Connection) -> Result<HashMap<String, HotkeyBinding>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT action, key_binding, description FROM hotkeys",
    )?;

    let hotkeys = stmt
        .query_map([], |row| {
            Ok(HotkeyBinding {
                action: row.get(0)?,
                key_binding: row.get(1)?,
                description: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .map(|h| (h.action.clone(), h))
        .collect();

    Ok(hotkeys)
}

pub fn update_hotkey(
    conn: &Connection,
    action: &str,
    key_binding: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE hotkeys SET key_binding = ?2, updated_at = datetime('now') WHERE action = ?1",
        rusqlite::params![action, key_binding],
    )?;
    Ok(())
}

pub fn parse_key_binding(binding: &str) -> Option<(KeyModifiers, KeyCode)> {
    let parts: Vec<&str> = binding.split('-').collect();

    let mut modifiers = KeyModifiers::empty();
    let key_str = if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            match *part {
                "C" | "Ctrl" => modifiers |= KeyModifiers::CONTROL,
                "S" | "Shift" => modifiers |= KeyModifiers::SHIFT,
                "A" | "Alt" => modifiers |= KeyModifiers::ALT,
                _ => {}
            }
        }
        parts[parts.len() - 1]
    } else {
        binding
    };

    let code = match key_str {
        "Enter" => KeyCode::Enter,
        "Escape" | "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" | "Del" => KeyCode::Delete,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        s if s.starts_with('F') => {
            s[1..].parse().ok().map(KeyCode::F)?
        }
        _ => return None,
    };

    Some((modifiers, code))
}
