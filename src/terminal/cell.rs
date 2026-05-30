use egui::Color32;

use crate::theme::terminal::TerminalTheme;

#[derive(Clone, Copy, PartialEq)]
pub struct CellAttrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
}

impl Default for CellAttrs {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            inverse: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TermColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl TermColor {
    pub fn to_color32(&self, is_fg: bool, theme: &TerminalTheme) -> Color32 {
        match self {
            TermColor::Default => {
                if is_fg {
                    theme.foreground
                } else {
                    theme.background
                }
            }
            TermColor::Indexed(idx) => theme.color_from_index(*idx),
            TermColor::Rgb(r, g, b) => Color32::from_rgb(*r, *g, *b),
        }
    }
}

#[derive(Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: TermColor,
    pub bg: TermColor,
    pub attrs: CellAttrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            attrs: CellAttrs::default(),
        }
    }
}
