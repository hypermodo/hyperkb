use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Cyberpunk,
    Modern,
    Nord,
    TokyoNight,
    Light,
}

impl ThemeMode {
    pub fn all() -> &'static [ThemeMode] {
        &[
            ThemeMode::Cyberpunk,
            ThemeMode::Modern,
            ThemeMode::Nord,
            ThemeMode::TokyoNight,
            ThemeMode::Light,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Cyberpunk => "Cyberpunk (Terminal Neon)",
            ThemeMode::Modern => "Modern (Electric Slate)",
            ThemeMode::Nord => "Nord (Arctic Frost)",
            ThemeMode::TokyoNight => "Tokyo Night (Storm & Lavender)",
            ThemeMode::Light => "Light (Paper & Indigo)",
        }
    }

    pub fn id_str(&self) -> &'static str {
        match self {
            ThemeMode::Cyberpunk => "cyberpunk",
            ThemeMode::Modern => "modern",
            ThemeMode::Nord => "nord",
            ThemeMode::TokyoNight => "tokyo_night",
            ThemeMode::Light => "light",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id.to_lowercase().as_str() {
            "modern" => ThemeMode::Modern,
            "nord" => ThemeMode::Nord,
            "tokyo_night" | "tokyonight" => ThemeMode::TokyoNight,
            "light" => ThemeMode::Light,
            _ => ThemeMode::Cyberpunk,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeMode::Cyberpunk => ThemeMode::Modern,
            ThemeMode::Modern => ThemeMode::Nord,
            ThemeMode::Nord => ThemeMode::TokyoNight,
            ThemeMode::TokyoNight => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Cyberpunk,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ThemeMode::Cyberpunk => ThemeMode::Light,
            ThemeMode::Modern => ThemeMode::Cyberpunk,
            ThemeMode::Nord => ThemeMode::Modern,
            ThemeMode::TokyoNight => ThemeMode::Nord,
            ThemeMode::Light => ThemeMode::TokyoNight,
        }
    }

    pub fn bg(&self) -> Color {
        match self {
            ThemeMode::Cyberpunk => Color::Rgb(10, 10, 16),
            ThemeMode::Modern => Color::Rgb(15, 23, 42),      // Slate-900
            ThemeMode::Nord => Color::Rgb(46, 52, 64),        // Polar Night Dark
            ThemeMode::TokyoNight => Color::Rgb(26, 27, 38),  // Storm
            ThemeMode::Light => Color::Rgb(248, 250, 252),    // Slate-50 Paper
        }
    }

    pub fn bg_panel(&self) -> Color {
        match self {
            ThemeMode::Cyberpunk => Color::Rgb(18, 18, 28),
            ThemeMode::Modern => Color::Rgb(30, 41, 59),      // Slate-800
            ThemeMode::Nord => Color::Rgb(59, 66, 82),        // Polar Night Medium
            ThemeMode::TokyoNight => Color::Rgb(36, 40, 59),  // Storm Medium
            ThemeMode::Light => Color::Rgb(255, 255, 255),    // Pure White
        }
    }

    pub fn accent(&self) -> Color {
        match self {
            ThemeMode::Cyberpunk => Color::Cyan,
            ThemeMode::Modern => Color::Rgb(56, 189, 248),    // Sky Blue (#38bdf8)
            ThemeMode::Nord => Color::Rgb(136, 192, 208),     // Frost Teal (#88c0d0)
            ThemeMode::TokyoNight => Color::Rgb(187, 154, 247), // Lavender (#bb9af7)
            ThemeMode::Light => Color::Rgb(37, 99, 235),       // Royal Blue (#2563eb)
        }
    }

    pub fn border(&self) -> Color {
        match self {
            ThemeMode::Cyberpunk => Color::Rgb(60, 60, 75),
            ThemeMode::Modern => Color::Rgb(51, 65, 85),      // Slate-700 (#334155)
            ThemeMode::Nord => Color::Rgb(76, 86, 106),       // Nord-3 (#4c566a)
            ThemeMode::TokyoNight => Color::Rgb(65, 72, 104), // Tokyo Night border (#414868)
            ThemeMode::Light => Color::Rgb(203, 213, 225),    // Slate-300 (#cbd5e1)
        }
    }

    pub fn border_focused(&self) -> Color {
        self.accent()
    }

    pub fn text_primary(&self) -> Color {
        match self {
            ThemeMode::Light => Color::Rgb(15, 23, 42),       // Dark Slate
            ThemeMode::Nord => Color::Rgb(236, 239, 244),     // Snow Storm White
            ThemeMode::TokyoNight => Color::Rgb(192, 202, 245),
            ThemeMode::Modern => Color::Rgb(241, 245, 249),
            ThemeMode::Cyberpunk => Color::White,
        }
    }

    pub fn text_muted(&self) -> Color {
        match self {
            ThemeMode::Light => Color::Rgb(100, 116, 139),    // Slate-500
            ThemeMode::Nord => Color::Rgb(143, 188, 187),     // Nord-8 Muted Teal
            ThemeMode::TokyoNight => Color::Rgb(122, 162, 247),
            ThemeMode::Modern => Color::Rgb(148, 163, 184),   // Slate-400
            ThemeMode::Cyberpunk => Color::Rgb(140, 140, 160),
        }
    }

    pub fn status_accepted(&self) -> Color {
        match self {
            ThemeMode::Cyberpunk => Color::Rgb(74, 222, 128), // Emerald Green (#4ade80)
            ThemeMode::Modern => Color::Rgb(52, 211, 153),    // Mint (#34d399)
            ThemeMode::Nord => Color::Rgb(163, 190, 140),     // Nord Green (#a3be8c)
            ThemeMode::TokyoNight => Color::Rgb(158, 206, 106),// Tokyo Green (#9ece6a)
            ThemeMode::Light => Color::Rgb(22, 163, 74),      // Forest Green (#16a34a)
        }
    }

    pub fn status_proposed(&self) -> Color {
        match self {
            ThemeMode::Nord => Color::Rgb(235, 203, 139),     // Nord Yellow (#ebcb8b)
            ThemeMode::TokyoNight => Color::Rgb(255, 158, 100),// Orange (#ff9e64)
            ThemeMode::Light => Color::Rgb(202, 138, 4),      // Amber (#ca8a04)
            ThemeMode::Modern => Color::Rgb(251, 191, 36),
            ThemeMode::Cyberpunk => Color::Rgb(250, 204, 21), // Amber (#facc15)
        }
    }

    pub fn status_risk(&self) -> Color {
        match self {
            ThemeMode::Nord => Color::Rgb(191, 97, 106),      // Nord Red (#bf616a)
            ThemeMode::TokyoNight => Color::Rgb(247, 118, 142),// Coral Red (#f7768e)
            ThemeMode::Light => Color::Rgb(220, 38, 38),      // Crimson (#dc2626)
            ThemeMode::Modern => Color::Rgb(248, 113, 113),
            ThemeMode::Cyberpunk => Color::Rgb(248, 113, 113),// Coral Red
        }
    }

    pub fn status_superseded(&self) -> Color {
        match self {
            ThemeMode::Light => Color::Rgb(148, 163, 184),
            ThemeMode::Nord => Color::Rgb(94, 129, 172),
            _ => Color::Rgb(156, 163, 175),
        }
    }

    pub fn status_unknown(&self) -> Color {
        match self {
            ThemeMode::Light => Color::Rgb(100, 116, 139),
            _ => Color::Rgb(148, 163, 184),
        }
    }

    pub fn title(&self) -> Style {
        Style::default().fg(self.accent()).add_modifier(Modifier::BOLD)
    }

    pub fn header(&self) -> Style {
        Style::default().fg(self.text_primary()).add_modifier(Modifier::BOLD)
    }

    pub fn selected_row(&self) -> Style {
        match self {
            ThemeMode::Cyberpunk => Style::default()
                .bg(Color::Rgb(30, 41, 59))
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            ThemeMode::Modern => Style::default()
                .bg(Color::Rgb(30, 58, 138))
                .fg(Color::Rgb(240, 249, 255))
                .add_modifier(Modifier::BOLD),
            ThemeMode::Nord => Style::default()
                .bg(Color::Rgb(67, 76, 94))
                .fg(Color::Rgb(136, 192, 208))
                .add_modifier(Modifier::BOLD),
            ThemeMode::TokyoNight => Style::default()
                .bg(Color::Rgb(46, 50, 77))
                .fg(Color::Rgb(187, 154, 247))
                .add_modifier(Modifier::BOLD),
            ThemeMode::Light => Style::default()
                .bg(Color::Rgb(219, 234, 254))
                .fg(Color::Rgb(30, 64, 175))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn active_tab(&self) -> Style {
        match self {
            ThemeMode::Light => Style::default()
                .bg(self.accent())
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            ThemeMode::Cyberpunk => Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            ThemeMode::Modern => Style::default()
                .bg(Color::Rgb(56, 189, 248))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            ThemeMode::Nord => Style::default()
                .bg(Color::Rgb(136, 192, 208))
                .fg(Color::Rgb(46, 52, 64))
                .add_modifier(Modifier::BOLD),
            ThemeMode::TokyoNight => Style::default()
                .bg(Color::Rgb(187, 154, 247))
                .fg(Color::Rgb(26, 27, 38))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn badge_accepted(&self) -> Style {
        Style::default()
            .fg(self.status_accepted())
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_proposed(&self) -> Style {
        Style::default()
            .fg(self.status_proposed())
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_risk(&self) -> Style {
        Style::default()
            .fg(self.status_risk())
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_acknowledged(&self) -> Style {
        Style::default()
            .fg(self.status_proposed())
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_resolved(&self) -> Style {
        Style::default()
            .fg(self.status_accepted())
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_conflict(&self) -> Style {
        Style::default()
            .fg(self.status_risk())
            .add_modifier(Modifier::BOLD)
    }

    pub fn key_badge(&self) -> Style {
        match self {
            ThemeMode::Light => Style::default().fg(Color::Rgb(37, 99, 235)).add_modifier(Modifier::BOLD),
            ThemeMode::Cyberpunk => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ThemeMode::Modern => Style::default().fg(Color::Rgb(56, 189, 248)).add_modifier(Modifier::BOLD),
            ThemeMode::Nord => Style::default().fg(Color::Rgb(235, 203, 139)).add_modifier(Modifier::BOLD),
            ThemeMode::TokyoNight => Style::default().fg(Color::Rgb(255, 158, 100)).add_modifier(Modifier::BOLD),
        }
    }

    pub fn section_header(&self) -> Style {
        Style::default().fg(self.accent()).add_modifier(Modifier::BOLD)
    }
}

/// Backward compatibility struct for static constants
pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Cyan;
    pub const BG: Color = Color::Reset;
    pub const TEXT: Color = Color::White;
    pub const TEXT_MUTED: Color = Color::DarkGray;
    pub const BORDER: Color = Color::Rgb(60, 60, 70);
    pub const BORDER_FOCUSED: Color = Color::Cyan;

    pub const STATUS_ACCEPTED: Color = Color::Rgb(74, 222, 128);
    pub const STATUS_PROPOSED: Color = Color::Rgb(250, 204, 21);
    pub const STATUS_SUPERSEDED: Color = Color::Rgb(156, 163, 175);
    pub const STATUS_RISK_OPEN: Color = Color::Rgb(248, 113, 113);
    pub const STATUS_UNKNOWN: Color = Color::Rgb(148, 163, 184);

    pub fn title() -> Style {
        Style::default().fg(Self::ACCENT).add_modifier(Modifier::BOLD)
    }

    pub fn header() -> Style {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    }

    pub fn selected_row() -> Style {
        Style::default()
            .bg(Color::Rgb(30, 41, 59))
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_accepted() -> Style {
        Style::default().fg(Self::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)
    }

    pub fn badge_proposed() -> Style {
        Style::default().fg(Self::STATUS_PROPOSED).add_modifier(Modifier::BOLD)
    }

    pub fn badge_risk() -> Style {
        Style::default().fg(Self::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD)
    }

    pub fn badge_acknowledged() -> Style {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    }

    pub fn badge_resolved() -> Style {
        Style::default().fg(Self::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)
    }

    pub fn badge_conflict() -> Style {
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_cycling() {
        let t = ThemeMode::Cyberpunk;
        assert_eq!(t.next(), ThemeMode::Modern);
        assert_eq!(t.next().next(), ThemeMode::Nord);
        assert_eq!(t.next().next().next(), ThemeMode::TokyoNight);
        assert_eq!(t.next().next().next().next(), ThemeMode::Light);
        assert_eq!(t.next().next().next().next().next(), ThemeMode::Cyberpunk);

        assert_eq!(t.prev(), ThemeMode::Light);
        assert_eq!(t.prev().prev(), ThemeMode::TokyoNight);
    }

    #[test]
    fn test_theme_mode_from_id() {
        assert_eq!(ThemeMode::from_id("modern"), ThemeMode::Modern);
        assert_eq!(ThemeMode::from_id("NORD"), ThemeMode::Nord);
        assert_eq!(ThemeMode::from_id("tokyo_night"), ThemeMode::TokyoNight);
        assert_eq!(ThemeMode::from_id("tokyonight"), ThemeMode::TokyoNight);
        assert_eq!(ThemeMode::from_id("light"), ThemeMode::Light);
        assert_eq!(ThemeMode::from_id("unknown"), ThemeMode::Cyberpunk);
    }
}
