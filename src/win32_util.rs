/// Windows 平台原生窗口操作工具（隐藏/显示/聚焦窗口）
/// 使用 Win32 API 实现，绕过 egui Visible(false) 停止 update() 的限制

#[cfg(target_os = "windows")]
pub mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HWND = isize;

    extern "system" {
        fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
        fn ShowWindow(hwnd: HWND, nCmdShow: i32) -> bool;
        fn SetForegroundWindow(hwnd: HWND) -> bool;
    }

    /// 将 Rust 字符串转为 Windows 宽字符串（以 null 结尾）
    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// 按窗口标题查找窗口句柄
    pub fn find_window(title: &str) -> HWND {
        let wide = to_wide(title);
        unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) }
    }

    /// 显示/隐藏窗口 (SW_HIDE=0, SW_RESTORE=9, SW_SHOW=5)
    pub fn show_window(hwnd: HWND, cmd: i32) {
        unsafe { ShowWindow(hwnd, cmd); }
    }

    /// 将窗口激活到前台
    pub fn set_foreground(hwnd: HWND) {
        unsafe { SetForegroundWindow(hwnd); }
    }
}

#[cfg(target_os = "windows")]
pub use platform::*;
