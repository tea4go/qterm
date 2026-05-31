use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

/// 获取应用配置目录路径
/// Windows: %APPDATA%\qterm
/// 其他: ~/.config/qterm
pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut p = PathBuf::from(appdata);
            p.push("qterm");
            return p;
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut p = PathBuf::from(home);
            p.push(".config");
            p.push("qterm");
            return p;
        }
    }
    PathBuf::from(".")
}

/// 获取配置文件完整路径（config.ini）
fn config_path() -> PathBuf {
    let mut path = config_dir();
    path.push("config.ini");
    path
}

/// 应用配置结构体
/// 存储窗口位置、尺寸、主题、字体大小等运行时配置
#[derive(Clone)]
pub struct AppConfig {
    pub window_x: Option<f32>,       // 窗口 X 坐标
    pub window_y: Option<f32>,       // 窗口 Y 坐标
    pub window_width: Option<f32>,   // 窗口宽度
    pub window_height: Option<f32>,  // 窗口高度
    pub maximized: bool,             // 是否最大化
    pub font_size: f32,              // 终端字体大小
    pub scrollback_lines: usize,     // 回滚缓冲区行数
    pub theme: String,               // 主题名称（dark/light）
    pub shell_path: String,          // 自定义 Shell 路径
    pub left_pane_width: f32,        // 左侧面板宽度
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            maximized: false,
            font_size: 14.0,
            scrollback_lines: 1000,
            theme: "dark".to_string(),
            shell_path: String::new(),
            left_pane_width: 220.0,
        }
    }
}

impl AppConfig {
    /// 从 config.ini 文件加载配置
    /// 如果文件不存在或解析失败，返回默认配置
    pub fn load() -> Self {
        let path = config_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let map = parse_ini(&content);
        Self {
            window_x: map.get("window_x").and_then(|v| v.parse().ok()),
            window_y: map.get("window_y").and_then(|v| v.parse().ok()),
            window_width: map.get("window_width").and_then(|v| v.parse().ok()),
            window_height: map.get("window_height").and_then(|v| v.parse().ok()),
            maximized: map.get("maximized").map(|v| v == "true").unwrap_or(false),
            font_size: map
                .get("font_size")
                .and_then(|v| v.parse().ok())
                .unwrap_or(14.0),
            scrollback_lines: map
                .get("scrollback_lines")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            theme: map
                .get("theme")
                .cloned()
                .unwrap_or_else(|| "dark".to_string()),
            shell_path: map.get("shell_path").cloned().unwrap_or_default(),
            left_pane_width: map
                .get("left_pane_width")
                .and_then(|v| v.parse().ok())
                .unwrap_or(220.0),
        }
    }

    /// 将当前配置保存到 config.ini 文件
    pub fn save(&self) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = config_path();
        let mut lines = Vec::new();
        if let Some(x) = self.window_x {
            lines.push(format!("window_x={}", x));
        }
        if let Some(y) = self.window_y {
            lines.push(format!("window_y={}", y));
        }
        if let Some(w) = self.window_width {
            lines.push(format!("window_width={}", w));
        }
        if let Some(h) = self.window_height {
            lines.push(format!("window_height={}", h));
        }
        lines.push(format!("maximized={}", self.maximized));
        lines.push(format!("font_size={}", self.font_size));
        lines.push(format!("scrollback_lines={}", self.scrollback_lines));
        lines.push(format!("theme={}", self.theme));
        if !self.shell_path.is_empty() {
            lines.push(format!("shell_path={}", self.shell_path));
        }
        lines.push(format!("left_pane_width={}", self.left_pane_width));
        let _ = std::fs::write(&path, lines.join("\n"));
    }
}

/// 解析 INI 格式配置文件
/// 简单的 key=value 格式，忽略注释行（#开头）和空行
fn parse_ini(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

// ==================== WhaleTerm preferences.json 配置 ====================

/// WhaleTerm preferences.json 文件结构（内部分析用）
#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct PreferencesFile {
    config: ConfigSection,
    general: GeneralSection,
    shell: ShellSection,
}

/// 配置区域字体设置（preferences.json config 部分）
#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct ConfigSection {
    default_font_family: Vec<String>,  // 默认字体族
    default_font_size: f32,            // 默认字体大小
    default_font_bold: String,         // 是否粗体
}

/// 通用 UI 字体设置（preferences.json general 部分）
#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct GeneralSection {
    font_family: Vec<String>,  // 通用字体族
    font_size: f32,            // 通用字体大小
    font_bold: String,         // 是否粗体
    theme: String,             // 主题名称
}

/// Shell/终端字体设置（preferences.json shell 部分）
#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct ShellSection {
    font_family: Vec<String>,  // 终端字体族
    font_size: f32,            // 终端字体大小
    font_bold: String,         // 是否粗体
}

/// 获取 WhaleTerm preferences.json 文件路径
/// Windows: %APPDATA%\WhaleTerm\preferences.json
/// 其他: ~/.config/WhaleTerm/preferences.json
fn preferences_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("WhaleTerm").join("preferences.json");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("WhaleTerm")
                .join("preferences.json");
        }
    }
    PathBuf::from("preferences.json")
}

/// 应用偏好设置结构体
/// 从 WhaleTerm preferences.json 读取字体和主题配置
#[derive(Clone)]
pub struct Preferences {
    pub config_font_family: Vec<String>,   // 配置区字体族
    pub config_font_size: f32,             // 配置区字体大小
    pub config_font_bold: bool,            // 配置区是否粗体
    pub general_font_family: Vec<String>,  // 通用区字体族
    pub general_font_size: f32,            // 通用区字体大小
    pub general_font_bold: bool,           // 通用区是否粗体
    pub shell_font_family: Vec<String>,    // 终端字体族
    pub shell_font_size: f32,              // 终端字体大小
    pub shell_font_bold: bool,             // 终端是否粗体
    pub theme: String,                     // 主题名称
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            config_font_family: vec![],
            config_font_size: 14.0,
            config_font_bold: false,
            general_font_family: vec![],
            general_font_size: 14.0,
            general_font_bold: false,
            shell_font_family: vec![],
            shell_font_size: 14.0,
            shell_font_bold: false,
            theme: "dark".to_string(),
        }
    }
}

impl Preferences {
    /// 从 WhaleTerm preferences.json 加载偏好设置
    /// 如果文件不存在或解析失败，返回默认设置
    pub fn load() -> Self {
        let path = preferences_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let pf: PreferencesFile = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };
        Self {
            config_font_family: pf.config.default_font_family,
            config_font_size: if pf.config.default_font_size > 0.0 {
                pf.config.default_font_size
            } else {
                14.0
            },
            config_font_bold: pf.config.default_font_bold == "bold",
            general_font_family: pf.general.font_family,
            general_font_size: if pf.general.font_size > 0.0 {
                pf.general.font_size
            } else {
                14.0
            },
            general_font_bold: pf.general.font_bold == "bold",
            shell_font_family: pf.shell.font_family,
            shell_font_size: if pf.shell.font_size > 0.0 {
                pf.shell.font_size
            } else {
                14.0
            },
            shell_font_bold: pf.shell.font_bold == "bold",
            theme: if pf.general.theme.is_empty() {
                "dark".to_string()
            } else {
                pf.general.theme
            },
        }
    }
}