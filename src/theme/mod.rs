pub mod extra;
pub mod system;
pub mod terminal;

use eframe::egui;

/// 主题模式（浅色/深色）
#[derive(Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,  // 浅色模式
    Dark,   // 深色模式
}

/// 应用主题结构体
/// 组合了系统主题（UI 颜色）、终端主题（ANSI 颜色）和扩展主题（进度条等）
pub struct AppTheme {
    pub mode: ThemeMode,               // 当前主题模式
    pub system: system::SystemTheme,   // 系统主题（UI 控件颜色）
    pub terminal: terminal::TerminalTheme, // 终端主题（ANSI 颜色、光标等）
    pub extra: extra::ExtraTheme,      // 扩展主题（SFTP进度条、表格等）
}

impl AppTheme {
    /// 创建深色主题（Solarized Dark）
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            system: system::SystemTheme::dark(),
            terminal: terminal::TerminalTheme::dark(),
            extra: extra::ExtraTheme::dark(),
        }
    }

    /// 创建浅色主题（Default Light Modern）
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            system: system::SystemTheme::light(),
            terminal: terminal::TerminalTheme::light(),
            extra: extra::ExtraTheme::light(),
        }
    }

    /// 设置主题模式
    pub fn set_mode(&mut self, mode: ThemeMode) {
        if self.mode != mode {
            *self = match mode {
                ThemeMode::Dark => Self::dark(),
                ThemeMode::Light => Self::light(),
            };
        }
    }

    /// 切换主题模式（浅色 ↔ 深色）
    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        });
    }

    /// 检查当前是否为深色模式
    pub fn is_dark(&self) -> bool {
        self.mode == ThemeMode::Dark
    }

    /// 获取终端字体大小
    pub fn font_size(&self) -> f32 {
        self.terminal.font_size
    }
}

/// 解析十六进制颜色字符串为 egui Color32
/// 格式：#RRGGBB
fn parse_color(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    egui::Color32::from_rgb(r, g, b)
}