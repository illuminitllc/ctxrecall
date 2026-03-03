use std::path::Path;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Pre-converted theme colors for use in render methods.
/// Avoids calling `parse_color()` on every frame.
pub struct ThemeStyles {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub selection: Color,
    pub border: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub muted: Color,
}

impl From<&Theme> for ThemeStyles {
    fn from(theme: &Theme) -> Self {
        Self {
            bg: Theme::parse_color(&theme.bg),
            fg: Theme::parse_color(&theme.fg),
            accent: Theme::parse_color(&theme.accent),
            selection: Theme::parse_color(&theme.selection),
            border: Theme::parse_color(&theme.border),
            error: Theme::parse_color(&theme.error),
            success: Theme::parse_color(&theme.success),
            warning: Theme::parse_color(&theme.warning),
            muted: Theme::parse_color(&theme.muted),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub selection: String,
    pub border: String,
    pub error: String,
    pub success: String,
    pub warning: String,
    pub muted: String,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            bg: "#1e1e2e".into(),
            fg: "#cdd6f4".into(),
            accent: "#89b4fa".into(),
            selection: "#313244".into(),
            border: "#585b70".into(),
            error: "#f38ba8".into(),
            success: "#a6e3a1".into(),
            warning: "#f9e2af".into(),
            muted: "#6c7086".into(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "light".into(),
            bg: "#eff1f5".into(),
            fg: "#4c4f69".into(),
            accent: "#1e66f5".into(),
            selection: "#ccd0da".into(),
            border: "#9ca0b0".into(),
            error: "#d20f39".into(),
            success: "#40a02b".into(),
            warning: "#df8e1d".into(),
            muted: "#8c8fa1".into(),
        }
    }

    pub fn solarized() -> Self {
        Self {
            name: "solarized".into(),
            bg: "#002b36".into(),
            fg: "#839496".into(),
            accent: "#268bd2".into(),
            selection: "#073642".into(),
            border: "#586e75".into(),
            error: "#dc322f".into(),
            success: "#859900".into(),
            warning: "#b58900".into(),
            muted: "#657b83".into(),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox".into(),
            bg: "#282828".into(),
            fg: "#ebdbb2".into(),
            accent: "#83a598".into(),
            selection: "#3c3836".into(),
            border: "#665c54".into(),
            error: "#fb4934".into(),
            success: "#b8bb26".into(),
            warning: "#fabd2f".into(),
            muted: "#928374".into(),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "nord".into(),
            bg: "#2e3440".into(),
            fg: "#d8dee9".into(),
            accent: "#88c0d0".into(),
            selection: "#3b4252".into(),
            border: "#4c566a".into(),
            error: "#bf616a".into(),
            success: "#a3be8c".into(),
            warning: "#ebcb8b".into(),
            muted: "#616e88".into(),
        }
    }

    pub fn last_horizon() -> Self {
        Self {
            name: "last-horizon".into(),
            bg: "#0c0b0c".into(),
            fg: "#e2dddc".into(),
            accent: "#b59790".into(),
            selection: "#3a2f30".into(),
            border: "#3a2f30".into(),
            error: "#c4d8e2".into(),
            success: "#87a9b0".into(),
            warning: "#c38b7b".into(),
            muted: "#9b7369".into(),
        }
    }

    pub fn builtin_themes() -> Vec<Theme> {
        vec![
            Self::dark(),
            Self::light(),
            Self::solarized(),
            Self::gruvbox(),
            Self::nord(),
            Self::last_horizon(),
        ]
    }

    pub fn parse_color(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Color::Reset;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    }

    /// Convenience: get pre-converted styles for use in render methods.
    pub fn styles(&self) -> ThemeStyles {
        ThemeStyles::from(self)
    }
}

/// Load a custom theme from `{config_dir}/theme.conf` (TOML format).
pub fn load_custom_theme(config_dir: &Path) -> Option<Theme> {
    let path = config_dir.join("theme.conf");
    let content = std::fs::read_to_string(&path).ok()?;
    let mut theme: Theme = toml::from_str(&content).ok()?;
    if theme.name.is_empty() {
        theme.name = "custom".into();
    }
    Some(theme)
}
