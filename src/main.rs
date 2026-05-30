#![allow(dead_code)]

mod app;
mod config;
mod pty;
mod tab;
mod terminal;
mod theme;
mod sftp;
mod ssh;
mod ui;

use eframe::egui;

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

#[cfg(not(windows))]
fn is_position_visible(x: f32, y: f32, _w: f32, _h: f32) -> bool {
    x >= 0.0 && y >= 0.0 && x < 5000.0 && y < 3000.0
}

fn main() -> eframe::Result<()> {
    let cfg = config::AppConfig::load();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("QTerm")
        .with_decorations(false)
        .with_min_inner_size([800.0, 500.0]);

    let w = cfg.window_width.unwrap_or(1100.0).max(800.0);
    let h = cfg.window_height.unwrap_or(700.0).max(500.0);
    viewport = viewport.with_inner_size([w, h]);

    if let (Some(x), Some(y)) = (cfg.window_x, cfg.window_y) {
        if is_position_visible(x, y, w, h) {
            viewport = viewport.with_position([x, y]);
        }
    }
    if cfg.maximized {
        viewport = viewport.with_maximized(true);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "QTerm",
        options,
        Box::new(move |cc| Ok(Box::new(app::QTermApp::new(cc, cfg)))),
    )
}
