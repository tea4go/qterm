pub mod extra;
pub mod system;
pub mod terminal;

use eframe::egui;

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

pub struct AppTheme {
    pub mode: ThemeMode,
    pub system: system::SystemTheme,
    pub terminal: terminal::TerminalTheme,
    pub extra: extra::ExtraTheme,
}

impl AppTheme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            system: system::SystemTheme::dark(),
            terminal: terminal::TerminalTheme::dark(),
            extra: extra::ExtraTheme::dark(),
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            system: system::SystemTheme::light(),
            terminal: terminal::TerminalTheme::light(),
            extra: extra::ExtraTheme::light(),
        }
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        if self.mode != mode {
            *self = match mode {
                ThemeMode::Dark => Self::dark(),
                ThemeMode::Light => Self::light(),
            };
        }
    }

    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        });
    }

    pub fn is_dark(&self) -> bool {
        self.mode == ThemeMode::Dark
    }

    pub fn font_size(&self) -> f32 {
        self.terminal.font_size
    }
}

fn parse_color(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    egui::Color32::from_rgb(r, g, b)
}
