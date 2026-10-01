use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    // Brand Colors
    pub const ACCENT: Color = Color::Cyan;
    pub const BG: Color = Color::Reset;
    pub const TEXT: Color = Color::White;
    pub const TEXT_MUTED: Color = Color::DarkGray;
    pub const BORDER: Color = Color::Rgb(60, 60, 70);
    pub const BORDER_FOCUSED: Color = Color::Cyan;

    // Semantic Status Colors
    pub const STATUS_ACCEPTED: Color = Color::Rgb(74, 222, 128); // Emerald Green
    pub const STATUS_PROPOSED: Color = Color::Rgb(250, 204, 21);  // Amber
    pub const STATUS_SUPERSEDED: Color = Color::Rgb(156, 163, 175); // Slate Gray
    pub const STATUS_RISK_OPEN: Color = Color::Rgb(248, 113, 113); // Coral Red
    pub const STATUS_UNKNOWN: Color = Color::Rgb(148, 163, 184);

    // Text Styles
    pub fn title() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row() -> Style {
        Style::default()
            .bg(Color::Rgb(30, 41, 59)) // Deep Slate
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_accepted() -> Style {
        Style::default()
            .fg(Self::STATUS_ACCEPTED)
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_proposed() -> Style {
        Style::default()
            .fg(Self::STATUS_PROPOSED)
            .add_modifier(Modifier::BOLD)
    }

    pub fn badge_risk() -> Style {
        Style::default()
            .fg(Self::STATUS_RISK_OPEN)
            .add_modifier(Modifier::BOLD)
    }
}
