/// 本地终端 Shell 类型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShellType {
    Cmd,        // Windows CMD
    PowerShell, // PowerShell
    GitBash,    // Git Bash
}

impl ShellType {
    /// 获取 Shell 显示名称
    pub fn label(&self) -> &'static str {
        match self {
            ShellType::Cmd => "CMD",
            ShellType::PowerShell => "PowerShell",
            ShellType::GitBash => "Git Bash",
        }
    }

    /// 获取 Shell 可执行文件路径
    pub fn shell_path(&self) -> Option<String> {
        match self {
            ShellType::Cmd => {
                // 优先使用 %COMSPEC%，回退到 cmd.exe
                Some(std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string()))
            }
            ShellType::PowerShell => {
                // 优先 pwsh（PowerShell 7+），回退 powershell
                Some(which("pwsh").unwrap_or_else(|| "powershell.exe".to_string()))
            }
            ShellType::GitBash => {
                // 查找 Git 安装目录下的 bash.exe
                find_git_bash()
            }
        }
    }

    /// 列出可用的 Shell 类型（仅返回系统中已安装的）
    pub fn available_shells() -> Vec<ShellType> {
        [ShellType::Cmd, ShellType::PowerShell, ShellType::GitBash]
            .iter()
            .filter(|s| s.shell_path().is_some())
            .copied()
            .collect()
    }
}

/// 在 PATH 中查找可执行文件（返回完整路径）
fn which(name: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let path_var = std::env::var("PATH").ok()?;
        for dir in path_var.split(';') {
            let full = std::path::Path::new(dir).join(format!("{}.exe", name));
            if full.exists() {
                return Some(full.to_string_lossy().to_string());
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let path_var = std::env::var("PATH").ok()?;
        for dir in path_var.split(':') {
            let full = std::path::Path::new(dir).join(name);
            if full.exists() {
                return Some(full.to_string_lossy().to_string());
            }
        }
        None
    }
}

/// 查找 Git Bash 的 bash.exe 路径
/// 搜索常见安装位置和 PATH
fn find_git_bash() -> Option<String> {
    // 常见 Git 安装路径
    let candidates: Vec<&str> = if cfg!(target_os = "windows") {
        vec![
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
            "C:\\Users\\%USERNAME%\\scoop\\apps\\git\\current\\bin\\bash.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec!["/usr/local/bin/bash", "/opt/homebrew/bin/bash"]
    } else {
        vec!["/usr/bin/bash"]
    };

    for path in &candidates {
        let expanded = if cfg!(target_os = "windows") {
            // 展开 %USERNAME%
            path.replace("%USERNAME%", &std::env::var("USERNAME").unwrap_or_default())
        } else {
            path.to_string()
        };
        if std::path::Path::new(&expanded).exists() {
            return Some(expanded);
        }
    }

    // 回退到 PATH 查找
    which("bash")
}

/// 获取系统默认 Shell 路径
/// Windows: 使用 %COMSPEC% 或 PowerShell
/// macOS: 使用 $SHELL 或 /bin/zsh
/// Linux: 使用 $SHELL 或 /bin/bash
pub fn default_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(comspec) = std::env::var("COMSPEC") {
            return comspec;
        }
        "powershell.exe".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}