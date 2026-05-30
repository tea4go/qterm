#![allow(dead_code)]

// 模块声明：各功能模块
mod app;       // 应用主逻辑
mod config;    // 配置管理
mod connection; // 连接管理（WhaleTerm 配置读取）
mod pty;       // 本地伪终端
mod tab;       // 标签页管理
mod terminal;  // 终端仿真器核心
mod theme;     // 主题系统
mod sftp;      // SFTP 文件传输
mod ssh;       // SSH 远程连接
mod ui;        // UI 组件（分屏、SFTP面板、SSH对话框）

use eframe::egui;

/// 检测窗口位置是否在可见的显示器范围内（Windows 版本）
/// 使用 Win32 API MonitorFromRect 判断指定矩形是否在任何显示器上
#[cfg(windows)]
fn is_position_visible(x: f32, y: f32, w: f32, h: f32) -> bool {
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    extern "system" {
        fn MonitorFromRect(lprc: *const RECT, dwFlags: u32) -> usize;
    }
    const MONITOR_DEFAULTTONULL: u32 = 0;
    let rc = RECT {
        left: x as i32,
        top: y as i32,
        right: (x + w) as i32,
        bottom: (y + h) as i32,
    };
    let monitor = unsafe { MonitorFromRect(&rc, MONITOR_DEFAULTTONULL) };
    monitor != 0
}

/// 检测窗口位置是否在可见范围内（非 Windows 版本）
/// 使用简单的坐标范围判断作为替代方案
#[cfg(not(windows))]
fn is_position_visible(x: f32, y: f32, _w: f32, _h: f32) -> bool {
    x >= 0.0 && y >= 0.0 && x < 5000.0 && y < 3000.0
}

/// 应用程序入口函数
/// 加载配置，创建窗口，启动 egui/eframe 渲染循环
fn main() -> eframe::Result<()> {
    let cfg = config::AppConfig::load();

    // 构建窗口视口：设置标题、最小尺寸、无原生装饰（自定义标题栏）
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("QTerm")
        .with_decorations(false)
        .with_min_inner_size([800.0, 500.0]);

    // 从配置恢复窗口尺寸（确保不低于最小值）
    let w = cfg.window_width.unwrap_or(1100.0).max(800.0);
    let h = cfg.window_height.unwrap_or(700.0).max(500.0);
    viewport = viewport.with_inner_size([w, h]);

    // 从配置恢复窗口位置（仅当位置在可见显示器范围内时）
    if let (Some(x), Some(y)) = (cfg.window_x, cfg.window_y) {
        if is_position_visible(x, y, w, h) {
            viewport = viewport.with_position([x, y]);
        }
    }
    // 从配置恢复最大化状态
    if cfg.maximized {
        viewport = viewport.with_maximized(true);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // 启动 eframe 渲染循环，创建 QTermApp 实例
    eframe::run_native(
        "QTerm",
        options,
        Box::new(move |cc| Ok(Box::new(app::QTermApp::new(cc, cfg)))),
    )
}