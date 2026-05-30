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