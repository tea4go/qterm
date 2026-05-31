use egui::Color32;

use crate::theme::terminal::TerminalTheme;

/// 单元格属性（粗体、斜体、下划线、删除线、反色）
#[derive(Clone, Copy, PartialEq)]
pub struct CellAttrs {
    pub bold: bool,            // 粗体
    pub italic: bool,          // 斜体
    pub underline: bool,       // 下划线
    pub strikethrough: bool,   // 删除线
    pub inverse: bool,         // 反色（交换前景和背景）
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

/// 终端颜色类型
/// 支持 Default（使用主题默认色）、Indexed（ANSI 256色索引）、Rgb（自定义RGB）
#[derive(Clone, Copy, PartialEq)]
pub enum TermColor {
    Default,            // 使用主题默认前景/背景色
    Indexed(u8),        // ANSI 16色或256色索引
    Rgb(u8, u8, u8),   // 自定义 RGB 颜色
}

impl TermColor {
    /// 将 TermColor 转换为 egui Color32
    /// is_fg 为 true 时作为前景色，false 时作为背景色
    pub fn to_color32(&self, is_fg: bool, theme: &TerminalTheme) -> Color32 {
        match self {
            TermColor::Default => {
                // Default 颜色根据前景/背景使用不同主题色
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

/// 终端单元格
/// 每个单元格存储一个字符及其前景色、背景色和显示属性
#[derive(Clone)]
pub struct Cell {
    pub ch: char,            // 字符内容
    pub fg: TermColor,       // 前景色
    pub bg: TermColor,       // 背景色
    pub attrs: CellAttrs,    // 显示属性
}

impl Default for Cell {
    /// 默认单元格：空格字符、默认颜色、无属性
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            attrs: CellAttrs::default(),
        }
    }
}