use std::collections::HashMap;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
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

fn config_path() -> PathBuf {
    let mut path = config_dir();
    path.push("config.ini");
    path
}

#[derive(Clone)]
pub struct AppConfig {
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    pub maximized: bool,
    pub font_size: f32,
    pub scrollback_lines: usize,
    pub theme: String,
    pub shell_path: String,
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
        }
    }
}

impl AppConfig {
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
        }
    }

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
        let _ = std::fs::write(&path, lines.join("\n"));
    }
}

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
