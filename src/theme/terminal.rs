use eframe::egui;

use super::parse_color;

pub struct TerminalTheme {
    pub font_size: f32,
    pub background: egui::Color32,
    pub foreground: egui::Color32,
    pub cursor: egui::Color32,
    pub cursor_accent: egui::Color32,
    pub selection_bg: egui::Color32,
    pub selection_fg: egui::Color32,
    pub ansi_colors: [egui::Color32; 16],
}

impl TerminalTheme {
    /// Solarized Dark — 对齐文档 §3.2
    pub fn dark() -> Self {
        Self {
            font_size: 14.0,
            background: parse_color("#002B36"),
            foreground: parse_color("#839496"),
            cursor: parse_color("#93A1A1"),
            cursor_accent: parse_color("#002B36"),
            selection_bg: parse_color("#073642"),
            selection_fg: parse_color("#93A1A1"),
            ansi_colors: [
                parse_color("#073642"), // black
                parse_color("#DC322F"), // red
                parse_color("#859900"), // green
                parse_color("#B58900"), // yellow
                parse_color("#268BD2"), // blue
                parse_color("#D33682"), // magenta
                parse_color("#2AA198"), // cyan
                parse_color("#EEE8D5"), // white
                parse_color("#002B36"), // bright black
                parse_color("#CB4B16"), // bright red
                parse_color("#586E75"), // bright green
                parse_color("#657B83"), // bright yellow
                parse_color("#839496"), // bright blue
                parse_color("#6C71C4"), // bright magenta
                parse_color("#93A1A1"), // bright cyan
                parse_color("#FDF6E3"), // bright white
            ],
        }
    }

    /// Default Light Modern — 对齐文档 §3.3
    pub fn light() -> Self {
        Self {
            font_size: 14.0,
            background: parse_color("#FFFFFF"),
            foreground: parse_color("#333333"),
            cursor: parse_color("#333333"),
            cursor_accent: parse_color("#FFFFFF"),
            selection_bg: parse_color("#ADD6FF"),
            selection_fg: parse_color("#000000"),
            ansi_colors: [
                parse_color("#000000"), // black
                parse_color("#CD3131"), // red
                parse_color("#429673"), // green
                parse_color("#949800"), // yellow
                parse_color("#0451A5"), // blue
                parse_color("#BC05BC"), // magenta
                parse_color("#009966"), // cyan
                parse_color("#A5A5A5"), // white
                parse_color("#666666"), // bright black
                parse_color("#CD3131"), // bright red
                parse_color("#429673"), // bright green
                parse_color("#949800"), // bright yellow
                parse_color("#0451A5"), // bright blue
                parse_color("#BC05BC"), // bright magenta
                parse_color("#009966"), // bright cyan
                parse_color("#A5A5A5"), // bright white
            ],
        }
    }

    pub fn color_from_index(&self, idx: u8) -> egui::Color32 {
        if idx < 16 {
            self.ansi_colors[idx as usize]
        } else if idx < 232 {
            let i = idx - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let to_val = |c: u8| if c == 0 { 0u8 } else { 55 + 40 * c };
            egui::Color32::from_rgb(to_val(r), to_val(g), to_val(b))
        } else {
            let gray = 8 + 10 * (idx - 232);
            egui::Color32::from_rgb(gray, gray, gray)
        }
    }
}
