//! Theme - Color schemes for the TUI
//!
//! Pre-baked themes for maximum eye-candy.

use ratatui::style::{Color, Modifier, Style};

/// Available color themes
#[derive(Debug, Clone, Copy, Default)]
pub enum Theme {
    #[default]
    Nord,
    Gruvbox,
    Dracula,
    Cyberpunk,
    Vibe,
}

/// Theme colors
pub struct ThemeColors {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
}

impl Theme {
    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::Nord => ThemeColors {
                bg: Color::Rgb(46, 52, 64),         // nord0
                fg: Color::Rgb(236, 239, 244),      // nord6
                accent: Color::Rgb(136, 192, 208),  // nord8 (frost)
                accent_dim: Color::Rgb(94, 129, 172), // nord10
                success: Color::Rgb(163, 190, 140), // nord14 (green)
                warning: Color::Rgb(235, 203, 139), // nord13 (yellow)
                error: Color::Rgb(191, 97, 106),    // nord11 (red)
                border: Color::Rgb(76, 86, 106),    // nord3
                header_bg: Color::Rgb(59, 66, 82),  // nord1
                header_fg: Color::Rgb(229, 233, 240), // nord5
                selected_bg: Color::Rgb(67, 76, 94), // nord2
                selected_fg: Color::Rgb(236, 239, 244), // nord6
            },
            Theme::Gruvbox => ThemeColors {
                bg: Color::Rgb(40, 40, 40),         // bg0
                fg: Color::Rgb(235, 219, 178),      // fg
                accent: Color::Rgb(131, 165, 152),  // aqua
                accent_dim: Color::Rgb(102, 92, 84), // gray
                success: Color::Rgb(152, 151, 26),  // green
                warning: Color::Rgb(215, 153, 33),  // yellow
                error: Color::Rgb(204, 36, 29),     // red
                border: Color::Rgb(80, 73, 69),     // bg2
                header_bg: Color::Rgb(60, 56, 54),  // bg1
                header_fg: Color::Rgb(235, 219, 178), // fg
                selected_bg: Color::Rgb(80, 73, 69), // bg2
                selected_fg: Color::Rgb(251, 241, 199), // fg0
            },
            Theme::Dracula => ThemeColors {
                bg: Color::Rgb(40, 42, 54),         // background
                fg: Color::Rgb(248, 248, 242),      // foreground
                accent: Color::Rgb(189, 147, 249),  // purple
                accent_dim: Color::Rgb(98, 114, 164), // comment
                success: Color::Rgb(80, 250, 123),  // green
                warning: Color::Rgb(241, 250, 140), // yellow
                error: Color::Rgb(255, 85, 85),     // red
                border: Color::Rgb(68, 71, 90),     // current line
                header_bg: Color::Rgb(68, 71, 90),  // current line
                header_fg: Color::Rgb(248, 248, 242), // foreground
                selected_bg: Color::Rgb(68, 71, 90), // current line
                selected_fg: Color::Rgb(80, 250, 123), // green
            },
            Theme::Cyberpunk => ThemeColors {
                bg: Color::Rgb(13, 2, 33),           // deep purple-black
                fg: Color::Rgb(224, 210, 255),       // soft lavender
                accent: Color::Rgb(199, 36, 177),    // neon magenta
                accent_dim: Color::Rgb(106, 90, 205), // slate blue
                success: Color::Rgb(0, 255, 159),    // neon green
                warning: Color::Rgb(255, 215, 0),    // gold
                error: Color::Rgb(255, 56, 100),     // hot pink
                border: Color::Rgb(138, 43, 226),    // blue-violet glow
                header_bg: Color::Rgb(30, 10, 60),   // dark violet
                header_fg: Color::Rgb(255, 105, 180), // hot pink
                selected_bg: Color::Rgb(75, 0, 130), // indigo
                selected_fg: Color::Rgb(0, 255, 255), // cyan glow
            },
            Theme::Vibe => ThemeColors {
                // Mistral Vibe inspired: Deep void black, cream text, sharp neon orange
                bg: Color::Rgb(10, 10, 10),           // Void Black
                fg: Color::Rgb(240, 238, 225),        // Cream / Off-white
                accent: Color::Rgb(255, 90, 0),       // Neon Vibe Orange
                accent_dim: Color::Rgb(120, 60, 20),  // Dimmed orange/brown
                success: Color::Rgb(100, 255, 100),   // Sharp Green
                warning: Color::Rgb(255, 200, 50),    // Sharp Yellow
                error: Color::Rgb(255, 50, 50),       // Sharp Red
                border: Color::Rgb(40, 40, 40),       // Subtle grey border
                header_bg: Color::Rgb(20, 20, 20),    // Slightly lighter black
                header_fg: Color::Rgb(255, 255, 255), // Pure white
                selected_bg: Color::Rgb(40, 20, 10),  // Very subtle orange tint
                selected_fg: Color::Rgb(255, 120, 0), // Orange text for selection
            },
        }
    }
}

/// Style shortcuts using theme
impl ThemeColors {
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn header(&self) -> Style {
        Style::default()
            .fg(self.header_fg)
            .bg(self.header_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn dim(&self) -> Style {
        Style::default().fg(self.accent_dim)
    }

    /// Success with background (for in-progress items)
    pub fn success_bg(&self) -> Style {
        Style::default()
            .fg(self.bg)
            .bg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Error with background (for blockers/overdue)
    pub fn error_bg(&self) -> Style {
        Style::default()
            .fg(self.bg)
            .bg(self.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Dim with subtle background (for completed items)
    pub fn done_bg(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.border)
    }
}
