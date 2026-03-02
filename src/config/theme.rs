use ratatui::style::Color;
use serde::{Deserialize, Serialize};

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

    pub fn builtin_themes() -> Vec<Theme> {
        vec![
            Self::dark(),
            Self::light(),
            Self::solarized(),
            Self::gruvbox(),
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
}
