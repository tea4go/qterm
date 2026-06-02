#![allow(dead_code)]
// debug 模式保留控制台窗口（方便 eprintln! 调试），release 模式隐藏控制台
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
#[cfg(target_os = "windows")]
mod win32_util; // Win32 原生窗口操作（隐藏/显示/聚焦）

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

/// Windows 平台设置 Per-Monitor DPI V2 感知
/// 必须在创建窗口前调用，确保多显示器 DPI 缩放正确
#[cfg(windows)]
fn set_dpi_awareness() {
    #[repr(C)]
    struct DPI_AWARENESS_CONTEXT(usize);
    extern "system" {
        fn SetProcessDpiAwarenessContext(value: DPI_AWARENESS_CONTEXT) -> i32;
    }
    const PER_MONITOR_AWARE_V2: DPI_AWARENESS_CONTEXT = DPI_AWARENESS_CONTEXT(-4isize as usize);
    unsafe {
        SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(windows))]
fn set_dpi_awareness() {}

// ==================== 启动日志 ====================

/// 获取 startup.log 文件路径
fn startup_log_path() -> std::path::PathBuf {
    let mut path = config::config_dir();
    path.push("startup.log");
    path
}

/// 写入一行日志到 startup.log
fn log_startup(msg: &str) {
    use std::io::Write;
    let path = startup_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{}] {}", timestamp, msg);
    }
}

/// 枚举所有显示器信息并写入日志（Windows 版本）
#[cfg(windows)]
fn log_monitors() {
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct MONITORINFOEX {
        size: u32,
        monitor: RECT,
        work: RECT,
        flags: u32,
        device: [u16; 32],
    }
    extern "system" {
        fn EnumDisplayMonitors(hdc: usize, lprcClip: usize, lpfnEnum: extern "system" fn(usize, usize, *mut RECT, isize) -> i32, dwData: isize) -> i32;
        fn GetMonitorInfoW(hMonitor: usize, lpmi: *mut MONITORINFOEX) -> i32;
    }

    let mut monitors_info: Vec<String> = Vec::new();
    let monitors_ptr = &mut monitors_info as *mut Vec<String> as isize;

    extern "system" fn enum_callback(h_monitor: usize, _hdc: usize, _lprc: *mut RECT, data: isize) -> i32 {
        let monitors = unsafe { &mut *(data as *mut Vec<String>) };
        let mut mi: MONITORINFOEX = unsafe { std::mem::zeroed() };
        mi.size = std::mem::size_of::<MONITORINFOEX>() as u32;
        unsafe {
            if GetMonitorInfoW(h_monitor, &mut mi) != 0 {
                let m = &mi.monitor;
                let w = &mi.work;
                let is_primary = mi.flags & 1 != 0;
                let tag = if is_primary { " [主屏]" } else { "" };
                let info = format!(
                    "  区域=({},{})~({},{}) 工作区=({},{})~({},{}){}",
                    m.left, m.top, m.right, m.bottom,
                    w.left, w.top, w.right, w.bottom,
                    tag
                );
                monitors.push(info);
            }
        }
        1
    }

    unsafe {
        EnumDisplayMonitors(0, 0, enum_callback, monitors_ptr);
    }

    log_startup(&format!("显示器 (共{}个):", monitors_info.len()));
    for info in &monitors_info {
        log_startup(info);
    }
}

/// 非 Windows 平台的显示器枚举（简化版本）
#[cfg(not(windows))]
fn log_monitors() {
    log_startup("显示器枚举仅支持 Windows 平台");
}

/// 获取主屏工作区中心坐标（物理像素），用于 --reset 居中
#[cfg(windows)]
fn primary_monitor_center(w: f32, h: f32) -> (f32, f32) {
    #[repr(C)]
    struct RECT { left: i32, top: i32, right: i32, bottom: i32 }
    extern "system" {
        fn SystemParametersInfoW(uiAction: u32, uiParam: u32, pvParam: *mut RECT, fWinIni: u32) -> i32;
    }
    const SPI_GETWORKAREA: u32 = 48;
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect, 0); }
    let work_w = (rect.right - rect.left) as f32;
    let work_h = (rect.bottom - rect.top) as f32;
    let cx = rect.left as f32 + (work_w - w) / 2.0;
    let cy = rect.top as f32 + (work_h - h) / 2.0;
    (cx.max(0.0), cy.max(0.0))
}

#[cfg(not(windows))]
fn primary_monitor_center(_w: f32, _h: f32) -> (f32, f32) {
    (100.0, 100.0)
}

/// 解析 --setpos x,y 命令行参数
fn parse_setpos(value: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() == 2 {
        let x = parts[0].parse::<f32>().ok()?;
        let y = parts[1].parse::<f32>().ok()?;
        Some((x, y))
    } else {
        None
    }
}

/// 应用程序入口函数
/// 加载配置，创建窗口，启动 egui/eframe 渲染循环
fn main() -> eframe::Result<()> {
    // 注意：不调用 set_dpi_awareness()，eframe/winit 已内置 DPI 处理

    // 启动日志
    log_startup("========== QTerm 启动 ==========");
    eprintln!("========== QTerm 启动 ==========");
    log_monitors();

    let cfg = config::AppConfig::load();

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let reset = args.iter().any(|a| a == "--reset");
    let setpos = args.windows(2)
        .find(|w| w[0] == "--setpos")
        .and_then(|w| parse_setpos(&w[1]));

    // 构建窗口视口：设置标题、最小尺寸、无原生装饰（自定义标题栏）
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("QTerm")
        .with_decorations(false)
        .with_min_inner_size([800.0, 500.0]);

    // 从配置恢复窗口尺寸（确保不低于最小值）
    let (w, h) = if reset {
        log_startup("--reset 模式，使用默认尺寸 1200x800");
        (1200.0f32, 800.0f32)
    } else {
        (cfg.window_width.unwrap_or(1100.0).max(800.0), cfg.window_height.unwrap_or(700.0).max(500.0))
    };
    viewport = viewport.with_inner_size([w, h]);

    log_startup(&format!(
        "配置: x={:?}, y={:?}, w={:?}, h={:?}, maximized={}",
        cfg.window_x, cfg.window_y, cfg.window_width, cfg.window_height, cfg.maximized
    ));

    // 计算目标窗口位置（延迟到第 2 帧设置，不在创建时定位）
    // 优先级：--reset > --setpos > 配置文件 > 系统默认
    let target_pos: Option<(f32, f32)> = if reset {
        let (cx, cy) = primary_monitor_center(w, h);
        log_startup(&format!("--reset 模式，主屏居中: ({}, {})", cx, cy));
        Some((cx, cy))
    } else if let Some(pos) = setpos {
        log_startup(&format!("--setpos 指定位置: ({}, {})", pos.0, pos.1));
        Some(pos)
    } else if let (Some(x), Some(y)) = (cfg.window_x, cfg.window_y) {
        let visible = is_position_visible(x, y, w, h);
        log_startup(&format!(
            "恢复窗口位置: 物理({}, {}), 大小({}, {}), 可见={}",
            x, y, w, h, visible
        ));
        if visible { Some((x, y)) } else {
            log_startup("位置不在可见区域，使用系统默认位置");
            None
        }
    } else {
        log_startup("无保存的位置，使用系统默认位置");
        None
    };

    // 从配置恢复最大化状态（最大化不受 DPI 影响，可在创建时设置）
    // --reset 时强制不最大化
    if cfg.maximized && !reset {
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
        Box::new(move |cc| Ok(Box::new(app::QTermApp::new(cc, cfg, target_pos)))),
    )
}