use std::path::PathBuf;

use eframe::egui;
use crate::config::{AppConfig, Preferences};
use crate::connection::models::Connection;
use crate::tab::Tab;
use crate::terminal::renderer;
use crate::theme::AppTheme;
use crate::ui::split_pane::{SplitDirection, PaneKind, PaneBackend};

// UI 常量尺寸
const TITLE_BAR_HEIGHT: f32 = 40.0;  // 标题栏高度
const LEFT_PANE_WIDTH: f32 = 220.0;   // 左侧面板宽度

/// QTerm 应用主结构体
/// 管理 UI 状态、标签页、配置、主题、SSH对话框等
pub struct QTermApp {
    tabs: Vec<Tab>,                       // 标签页列表
    active_tab: usize,                    // 当前活动标签页索引
    config: AppConfig,                    // 应用配置
    preferences: Preferences,             // 偏好设置（字体、主题）
    theme: AppTheme,                      // 当前主题
    last_window_pos: Option<(f32, f32)>,  // 上次窗口位置（物理像素）
    last_window_size: Option<(f32, f32)>, // 上次窗口尺寸（egui points）
    last_maximized: bool,                 // 上次最大化状态
    last_cols: usize,                     // 上次终端列数
    last_rows: usize,                     // 上次终端行数
    frame_count: u32,                     // 帧计数器（用于延迟定位）
    target_physical_pos: Option<(f32, f32)>, // 目标窗口位置（物理像素，第 2 帧应用）
    ssh_dialog: crate::ui::ssh_dialog::SshDialog,  // SSH 连接对话框
    sftp_error: Option<String>,           // SFTP 错误信息
    show_left_pane: bool,                 // 是否显示左侧面板
    context_menu: ContextMenu,            // 右键上下文菜单
    pending_mouse: Option<PendingMouse>,  // 待处理的鼠标事件
    connections: Vec<Connection>,          // WhaleTerm 连接列表
    connections_rx: Option<std::sync::mpsc::Receiver<Vec<Connection>>>,
    selected_connection: Option<usize>,    // 当前选中的连接索引
    collapsed_groups: std::collections::HashSet<String>, // 收缩的分组名集合
}

/// 右键上下文菜单状态
struct ContextMenu {
    show: bool,       // 是否显示菜单
    pos: egui::Pos2,  // 菜单显示位置
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self { show: false, pos: egui::Pos2::ZERO }
    }
}

/// 待处理的鼠标事件数据
/// 用于终端区域的拖拽选择和双击选择等交互
struct PendingMouse {
    response: egui::Response,   // 鼠标响应对象
    cell_width: f32,            // 单元格宽度
    cell_height: f32,           // 单元格高度
    origin: egui::Pos2,        // 绘制起点坐标
}

impl QTermApp {
    /// 创建 QTermApp 实例
    /// 初始化字体、主题、偏好设置，创建第一个本地终端标签页
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig, target_pos: Option<(f32, f32)>) -> Self {
        let preferences = Preferences::load();
        Self::configure_fonts(&cc.egui_ctx, &preferences);
        let is_dark = preferences.theme != "light";
        let theme = if is_dark { AppTheme::dark() } else { AppTheme::light() };
        let font_size = preferences.shell_font_size;
        theme.system.apply_to_egui(&cc.egui_ctx, is_dark, preferences.general_font_size);
        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            config,
            preferences,
            theme,
            last_window_pos: None,
            last_window_size: None,
            last_maximized: false,
            last_cols: 80,
            last_rows: 24,
            frame_count: 0,
            target_physical_pos: target_pos,
            ssh_dialog: crate::ui::ssh_dialog::SshDialog::new(),
            sftp_error: None,
            show_left_pane: true,
            context_menu: ContextMenu::default(),
            pending_mouse: None,
            connections: Vec::new(),
            selected_connection: None,
            collapsed_groups: std::collections::HashSet::new(),
            connections_rx: {
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let conns = crate::connection::load_connections();
                    let _ = tx.send(conns);
                });
                Some(rx)
            },
        };
        app.theme.terminal.font_size = font_size;
        app.theme.terminal.font_bold = app.preferences.shell_font_bold;
        app.config.font_size = font_size;
        app.new_tab();
        app
    }

    /// 配置 egui 字体系统
    /// 加载用户配置的字体族和系统回退字体（CJK + 等宽）
    fn configure_fonts(ctx: &egui::Context, prefs: &Preferences) {
        let mut fonts = egui::FontDefinitions::default();

        // 收集所有唯一字体族名称（跨三个配置区域）
        let mut all_families: Vec<String> = Vec::new();
        for name in &prefs.config_font_family {
            if !all_families.contains(name) {
                all_families.push(name.clone());
            }
        }
        for name in &prefs.general_font_family {
            if !all_families.contains(name) {
                all_families.push(name.clone());
            }
        }
        for name in &prefs.shell_font_family {
            if !all_families.contains(name) {
                all_families.push(name.clone());
            }
        }

        // 加载用户配置的字体文件
        for name in &all_families {
            for path in find_font_paths(name) {
                if let Ok(data) = std::fs::read(&path) {
                    fonts.font_data.insert(
                        path.clone(),
                        egui::FontData::from_owned(data).into(),
                    );
                    fonts.families.entry(egui::FontFamily::Proportional).or_default().push(path.clone());
                    fonts.families.entry(egui::FontFamily::Monospace).or_default().push(path.clone());
                    break;
                }
            }
        }

        // 系统回退字体路径（CJK 支持 + 等宽字体）
        let fallback_paths: Vec<&str> = if cfg!(target_os = "windows") {
            vec!["C:\\Windows\\Fonts\\msyh.ttc", "C:\\Windows\\Fonts\\consola.ttf"]
        } else if cfg!(target_os = "macos") {
            vec!["/System/Library/Fonts/PingFang.ttc"]
        } else {
            vec!["/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"]
        };

        // 加载回退字体（避免重复加载）
        for path in &fallback_paths {
            if fonts.font_data.contains_key(*path) {
                continue;
            }
            if let Ok(data) = std::fs::read(*path) {
                fonts.font_data.insert(
                    path.to_string(),
                    egui::FontData::from_owned(data).into(),
                );
                fonts.families.entry(egui::FontFamily::Proportional).or_default().push(path.to_string());
                fonts.families.entry(egui::FontFamily::Monospace).or_default().push(path.to_string());
            }
        }

        // 注册命名字体族 "general"，用于左侧面板
        // 粗体时优先使用粗体字体变体
        let general_family = egui::FontFamily::Name(std::sync::Arc::from("general"));
        let mut general_fonts_list: Vec<String> = Vec::new();
        if prefs.general_font_bold {
            let bold_path = if cfg!(target_os = "windows") {
                "C:\\Windows\\Fonts\\msyhbd.ttc"
            } else if cfg!(target_os = "macos") {
                "/System/Library/Fonts/PingFang.ttc"
            } else {
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Bold.ttc"
            };
            if !fonts.font_data.contains_key(bold_path) {
                if let Ok(data) = std::fs::read(bold_path) {
                    fonts.font_data.insert(
                        bold_path.to_string(),
                        egui::FontData::from_owned(data).into(),
                    );
                }
            }
            if fonts.font_data.contains_key(bold_path) {
                general_fonts_list.push(bold_path.to_string());
            }
        }
        // 复制 Proportional 中的所有字体作为回退
        if let Some(prop_fonts) = fonts.families.get(&egui::FontFamily::Proportional) {
            for key in prop_fonts {
                if !general_fonts_list.contains(key) {
                    general_fonts_list.push(key.clone());
                }
            }
        }
        fonts.families.insert(general_family, general_fonts_list);

        ctx.set_fonts(fonts);
    }

    /// 获取配置区字体 ID
    fn config_font_id(&self) -> egui::FontId {
        egui::FontId::proportional(self.preferences.config_font_size)
    }

    /// 获取通用字体 ID（可指定大小）
    fn general_font_id(&self, size: Option<f32>) -> egui::FontId {
        let sz = size.unwrap_or(self.preferences.general_font_size);
        egui::FontId::proportional(sz)
    }

    /// 获取终端字体 ID（等宽字体）
    fn shell_font_id(&self) -> egui::FontId {
        egui::FontId::monospace(self.preferences.shell_font_size)
    }

    /// 获取左侧面板字体 ID（使用 general 命名字体族，支持粗体）
    fn sidebar_font_id(&self, size: f32) -> egui::FontId {
        egui::FontId::new(size, egui::FontFamily::Name(std::sync::Arc::from("general")))
    }

    /// 使用 sidebar 字体渲染标签（绕过 egui 0.29 RichText.font() 的 bug）
    fn sidebar_label(&self, ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
        let font_id = self.sidebar_font_id(size);
        let galley = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font_id, color));
        ui.label(galley);
    }

    /// 创建新的本地终端标签页
    fn new_tab(&mut self) {
        let shell = if self.config.shell_path.is_empty() {
            None
        } else {
            Some(self.config.shell_path.as_str())
        };
        match Tab::new_local(self.last_rows, self.last_cols, self.config.scrollback_lines, shell) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
            }
            Err(e) => {
                eprintln!("创建标签页失败: {}", e);
            }
        }
    }

    /// 关闭指定索引的标签页
    fn close_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.tabs[idx].close();
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }
}

/// 根据字体名称查找系统字体文件路径
/// 支持 Windows、macOS、Linux 三平台的字体目录
fn find_font_paths(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let base = lower.replace(' ', "");
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        let font_dir = "C:\\Windows\\Fonts\\";
        // 用户字体目录（%LOCALAPPDATA%\Fonts）
        let user_font_dir = std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Fonts").to_string_lossy().to_string())
            .unwrap_or_default();

        // 尝试多种扩展名和粗体变体
        for ext in &["ttf", "ttc", "otf"] {
            paths.push(format!("{}{}.{}", font_dir, base, ext));
            paths.push(format!("{}{}bd.{}", font_dir, base, ext));
            paths.push(format!("{}{}bold.{}", font_dir, base, ext));
        }
        if !user_font_dir.is_empty() {
            for ext in &["ttf", "ttc", "otf"] {
                paths.push(format!("{}\\{}.{}", user_font_dir, base, ext));
            }
        }
    } else if cfg!(target_os = "macos") {
        for dir in &["/System/Library/Fonts/", "/Library/Fonts/"] {
            for ext in &["ttf", "ttc", "otf"] {
                paths.push(format!("{}{}.{}", dir, base, ext));
            }
        }
    } else {
        for dir in &[
            "/usr/share/fonts/truetype/",
            "/usr/share/fonts/opentype/",
            "/usr/local/share/fonts/",
        ] {
            for ext in &["ttf", "ttc", "otf"] {
                paths.push(format!("{}{}.{}", dir, base, ext));
            }
        }
    }

    paths
}

/// 用户操作类型枚举
/// 用于处理全局快捷键映射
enum Action {
    NewTab,           // 新建标签页
    CloseTab,         // 关闭标签页
    NextTab,          // 切换到下一个标签页
    SplitHorizontal,  // 水平分屏
    SplitVertical,    // 垂直分屏
    NextPane,         // 切换到下一个面板
    ClosePane,        // 关闭活动面板
    OpenSshDialog,    // 打开 SSH 连接对话框
    OpenSftp,         // 打开 SFTP
    ToggleLeftPane,   // 切换左侧面板显示
    FontZoomIn,       // 字体放大
    FontZoomOut,      // 字体缩小
}

// ==================== eframe::App 实现 ====================

impl eframe::App for QTermApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 第 2 帧延迟定位窗口（第 1 帧 DPI 缩放因子尚未准确）
        self.frame_count += 1;
        if self.frame_count == 2 {
            if let Some((px, py)) = self.target_physical_pos.take() {
                let ppp = ctx.pixels_per_point();
                let pos = egui::pos2(px / ppp, py / ppp);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
        }

        // 接收后台线程加载的连接列表
        if let Some(rx) = self.connections_rx.take() {
            match rx.try_recv() {
                Ok(conns) => self.connections = conns,
                Err(std::sync::mpsc::TryRecvError::Empty) => self.connections_rx = Some(rx),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
            }
        }

        // 轮询所有标签页（读取终端输出数据）
        for tab in &mut self.tabs {
            tab.poll();
        }

        // 记录窗口位置和尺寸（位置转物理像素，尺寸保持 egui points）
        let ppp = ctx.pixels_per_point();
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.last_window_size = Some((rect.width(), rect.height()));
            }
            if let Some(rect) = i.viewport().outer_rect {
                self.last_window_pos = Some((rect.min.x * ppp, rect.min.y * ppp));
            }
            self.last_maximized = i.viewport().maximized.unwrap_or(false);
        });

        // 处理全局快捷键
        let mut action = None;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::H) && i.modifiers.ctrl && i.modifiers.shift {
                action = Some(Action::SplitHorizontal);
            }
            if i.key_pressed(egui::Key::V) && i.modifiers.ctrl && i.modifiers.shift {
                action = Some(Action::SplitVertical);
            }
            if i.key_pressed(egui::Key::W) && i.modifiers.ctrl && i.modifiers.shift {
                action = Some(Action::ClosePane);
            }
            if i.key_pressed(egui::Key::N) && i.modifiers.ctrl && i.modifiers.shift {
                action = Some(Action::OpenSshDialog);
            }
            if i.key_pressed(egui::Key::F) && i.modifiers.ctrl && i.modifiers.shift {
                action = Some(Action::OpenSftp);
            }
            if i.key_pressed(egui::Key::ArrowRight) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::NextPane);
            }
            if i.key_pressed(egui::Key::ArrowDown) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::NextPane);
            }
            if i.key_pressed(egui::Key::T) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::NewTab);
            }
            if i.key_pressed(egui::Key::W) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::CloseTab);
            }
            if i.key_pressed(egui::Key::Tab) && i.modifiers.ctrl {
                action = Some(Action::NextTab);
            }
            if i.key_pressed(egui::Key::B) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::ToggleLeftPane);
            }
            if (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus)) && i.modifiers.ctrl {
                action = Some(Action::FontZoomIn);
            }
            if i.key_pressed(egui::Key::Minus) && i.modifiers.ctrl {
                action = Some(Action::FontZoomOut);
            }
        });
        // 执行快捷键对应的操作
        match action {
            Some(Action::NewTab) => self.new_tab(),
            Some(Action::CloseTab) => { let idx = self.active_tab; self.close_tab(idx); }
            Some(Action::NextTab) => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
            }
            Some(Action::SplitHorizontal) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let shell = if self.config.shell_path.is_empty() { None } else { Some(self.config.shell_path.as_str()) };
                    let _ = tab.layout.add_local_pane(SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines, shell);
                }
            }
            Some(Action::SplitVertical) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let shell = if self.config.shell_path.is_empty() { None } else { Some(self.config.shell_path.as_str()) };
                    let _ = tab.layout.add_local_pane(SplitDirection::Vertical, self.last_rows, self.last_cols, self.config.scrollback_lines, shell);
                }
            }
            Some(Action::NextPane) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let count = tab.layout.pane_count();
                    if count > 0 {
                        tab.layout.active_pane = (tab.layout.active_pane + 1) % count;
                    }
                }
            }
            Some(Action::ClosePane) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let idx = tab.layout.active_pane;
                    tab.layout.remove_pane(idx);
                }
            }
            Some(Action::OpenSshDialog) => { self.ssh_dialog.open = true; }
            Some(Action::OpenSftp) => { self.handle_open_sftp(); }
            Some(Action::ToggleLeftPane) => { self.show_left_pane = !self.show_left_pane; }
            Some(Action::FontZoomIn) => {
                self.config.font_size = (self.config.font_size + 1.0).min(30.0);
                self.theme.terminal.font_size = self.config.font_size;
                Self::configure_fonts(ctx, &self.preferences);
            }
            Some(Action::FontZoomOut) => {
                self.config.font_size = (self.config.font_size - 1.0).max(11.0);
                self.theme.terminal.font_size = self.config.font_size;
                Self::configure_fonts(ctx, &self.preferences);
            }
            None => {}
        }

        // === 标题栏渲染（40px） ===
        self.render_title_bar(ctx);

        // === 左侧面板：连接列表 ===
        if self.show_left_pane {
            let left_bg = self.theme.system.app_left_list_bg_color;
            egui::SidePanel::left("left_panel")
                .frame(egui::Frame::none().fill(self.theme.system.app_left_list_bg_color))
                .exact_width(LEFT_PANE_WIDTH)
                .show_separator_line(false)
                .show(ctx, |ui| {
                    self.render_left_pane(ui);
                });
        }

        // === 底部状态栏 ===
        self.render_foot_bar(ctx);

        // === 中央面板：终端区域 ===
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.terminal.background))
            .show(ctx, |ui| {
                if self.tabs.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("按 Ctrl+T 打开新终端");
                    });
                    return;
                }

                // 计算当前活动标签页的面板数和终端尺寸
                let pane_count = {
                    let tab = &self.tabs[self.active_tab];
                    tab.layout.pane_count()
                };

                let size = renderer::calculate_size(ui, self.theme.terminal.font_size);
                // 根据分屏方向计算每个面板的目标行数和列数
                let (target_rows, target_cols) = if pane_count <= 1 {
                    (size.rows, size.cols)
                } else {
                    let tab = &self.tabs[self.active_tab];
                    match tab.layout.direction {
                        SplitDirection::Horizontal => ((size.rows / pane_count).max(1), size.cols),
                        SplitDirection::Vertical => (size.rows, (size.cols / pane_count).max(1)),
                    }
                };
                // 尺寸变化时调整所有面板的终端大小
                if target_rows != self.last_rows || target_cols != self.last_cols {
                    self.last_rows = target_rows;
                    self.last_cols = target_cols;
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        for pane in &mut tab.layout.panes {
                            pane.resize(target_rows, target_cols);
                        }
                    }
                }

                // 渲染终端面板内容
                let tab = &mut self.tabs[self.active_tab];
                if pane_count <= 1 {
                    // 单面板模式
                    if let Some(pane) = tab.layout.active_pane_mut() {
                        match &mut pane.kind {
                            PaneKind::Terminal { terminal, .. } => {
                                let rr = renderer::render(ui, terminal, &self.theme.terminal);
                                self.pending_mouse = Some(PendingMouse {
                                    response: rr.response,
                                    cell_width: rr.cell_width,
                                    cell_height: rr.cell_height,
                                    origin: rr.origin,
                                });
                            }
                            PaneKind::Sftp { panel } => {
                                panel.show(ui);
                            }
                        }
                    }
                } else {
                    // 多面板分屏模式
                    let active_idx = tab.layout.active_pane;
                    match tab.layout.direction {
                        SplitDirection::Horizontal => {
                            // 水平分屏：上下排列
                            let available_height = ui.available_height();
                            let pane_height = available_height / pane_count as f32;
                            for (idx, pane) in tab.layout.panes.iter_mut().enumerate() {
                                let is_active = idx == active_idx;
                                // 活动面板显示边框高亮
                                let stroke = if is_active {
                                    egui::Stroke::new(1.0, self.theme.system.text_active_color)
                                } else {
                                    egui::Stroke::NONE
                                };
                                egui::Frame::none()
                                    .fill(self.theme.terminal.background)
                                    .stroke(stroke)
                                    .show(ui, |ui| {
                                        ui.set_max_height(pane_height - 2.0);
                                        match &mut pane.kind {
                                            PaneKind::Terminal { terminal, .. } => {
                                                let rr = renderer::render(ui, terminal, &self.theme.terminal);
                                                if is_active {
                                                    self.pending_mouse = Some(PendingMouse {
                                                        response: rr.response,
                                                        cell_width: rr.cell_width,
                                                        cell_height: rr.cell_height,
                                                        origin: rr.origin,
                                                    });
                                                }
                                            }
                                            PaneKind::Sftp { panel } => {
                                                panel.show(ui);
                                            }
                                        }
                                    });
                            }
                        }
                        SplitDirection::Vertical => {
                            // 垂直分屏：左右排列
                            let available_width = ui.available_width();
                            let pane_width = available_width / pane_count as f32;
                            ui.horizontal(|ui| {
                                for (idx, pane) in tab.layout.panes.iter_mut().enumerate() {
                                    let is_active = idx == active_idx;
                                    let stroke = if is_active {
                                        egui::Stroke::new(1.0, self.theme.system.text_active_color)
                                    } else {
                                        egui::Stroke::NONE
                                    };
                                    egui::Frame::none()
                                        .fill(self.theme.terminal.background)
                                        .stroke(stroke)
                                        .show(ui, |ui| {
                                            ui.set_max_width(pane_width - 2.0);
                                            match &mut pane.kind {
                                                PaneKind::Terminal { terminal, .. } => {
                                                    let rr = renderer::render(ui, terminal, &self.theme.terminal);
                                                    if is_active {
                                                        self.pending_mouse = Some(PendingMouse {
                                                            response: rr.response,
                                                            cell_width: rr.cell_width,
                                                            cell_height: rr.cell_height,
                                                            origin: rr.origin,
                                                        });
                                                    }
                                                }
                                                PaneKind::Sftp { panel } => {
                                                    panel.show(ui);
                                                }
                                            }
                                        });
                                }
                            });
                        }
                    }
                }
            });

        // 显示 SSH 连接对话框
        self.ssh_dialog.show(ctx);

        // 处理 SSH 连接结果
        if let Some(config) = self.ssh_dialog.result.take() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                if let Err(e) = tab.layout.add_ssh_pane(config, SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines) {
                    self.ssh_dialog.status = Some(format!("SSH 错误: {}", e));
                    self.ssh_dialog.open = true;
                }
            }
        }

        // 处理用户输入（键盘、鼠标）
        self.handle_input(ctx);
        ctx.request_repaint();
    }

    /// 应用退出时保存配置并关闭所有标签页
    fn on_exit(&mut self) {
        self.config.window_x = self.last_window_pos.map(|(x, _)| x);
        self.config.window_y = self.last_window_pos.map(|(_, y)| y);
        self.config.window_width = self.last_window_size.map(|(w, _)| w);
        self.config.window_height = self.last_window_size.map(|(_, h)| h);
        self.config.maximized = self.last_maximized;
        self.config.theme = if self.theme.is_dark() { "dark".to_string() } else { "light".to_string() };
        self.config.save();
        for tab in &mut self.tabs {
            tab.close();
        }
    }
}

// ==================== 标题栏渲染 ====================

impl QTermApp {
    /// 渲染自定义标题栏
    /// 包含：窗口拖拽区、标签页列表、窗口控制按钮（最小化/最大化/关闭）
    fn render_title_bar(&mut self, ctx: &egui::Context) {
        let title_bar_h = TITLE_BAR_HEIGHT;
        let app_bg = self.theme.system.app_bg_color;
        let header_text = self.theme.system.app_header_text_color;
        let text_active = self.theme.system.text_active_color;
        let hover_bg = self.theme.system.app_side_hover_bg_color;
        let side_text = self.theme.system.app_side_text_color;
        let text_color = self.theme.system.text_color;
        let is_maximized = self.last_maximized;

        egui::TopBottomPanel::top("title_bar")
            .frame(egui::Frame::none().fill(app_bg))
            .exact_height(title_bar_h)
            .show(ctx, |ui| {
                // 窗口拖拽区域（双击切换最大化）
                let title_bar_response = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("title_bar_drag"),
                    egui::Sense::click_and_drag(),
                );
                if title_bar_response.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    self.last_maximized = !is_maximized;
                }
                if title_bar_response.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // 最小化 / 最大化 / 关闭按钮（右上角）
                let btn_w = 32.0;
                let btn_h = title_bar_h;
                let total_btn_w = btn_w * 3.0;
                let right_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(ui.max_rect().right() - total_btn_w, ui.max_rect().top()),
                    egui::vec2(total_btn_w, btn_h),
                );

                // 左侧：标题 + 标签页
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("QTerm").font(egui::FontId::proportional(18.0)).strong().color(header_text));
                    ui.add_space(16.0);

                    // 卡片风格标签页
                    let mut close_idx = None;
                    let tab_h = title_bar_h - 6.0;
                    for (idx, tab) in self.tabs.iter().enumerate() {
                        let selected = idx == self.active_tab;
                        let label = if tab.alive() { &tab.title } else { "[已关闭]" };

                        let (text_color, bg_color) = if selected {
                            (text_active, hover_bg)
                        } else {
                            (side_text, egui::Color32::TRANSPARENT)
                        };

                        let tab_frame = egui::Frame::none()
                            .fill(bg_color)
                            .rounding(egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 });

                        let inner = tab_frame.show(ui, |ui| {
                            ui.set_min_height(tab_h);
                            ui.set_max_height(tab_h);
                            ui.horizontal(|ui| {
                                ui.add_space(5.0);
                                ui.label(egui::RichText::new(label).color(text_color).size(13.0));
                                ui.add_space(4.0);
                                // 关闭标签按钮
                                let close_btn = ui.add(egui::Button::new(
                                    egui::RichText::new("x").size(11.0).color(text_color),
                                ).frame(false).min_size(egui::vec2(30.0, 20.0)));
                                if close_btn.clicked() {
                                    close_idx = Some(idx);
                                }
                                ui.add_space(4.0);
                            });
                        });

                        // 点击标签页切换活动标签
                        if inner.response.clicked() {
                            self.active_tab = idx;
                        }
                        ui.add_space(1.0);
                    }

                    // "+" 新建标签按钮
                    if ui.add(egui::Button::new(
                        egui::RichText::new("+").size(16.0).color(side_text),
                    ).frame(false)).clicked() {
                        self.new_tab();
                    }

                    if let Some(idx) = close_idx {
                        self.close_tab(idx);
                    }
                });

                // 窗口控制按钮（右上角绝对定位）
                use egui::ViewportCommand;
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let hover_bg = egui::Color32::from_white_alpha(20);
                        let close_hover_bg = egui::Color32::from_rgb(232, 17, 35);
                        let btn_size = egui::vec2(btn_w, btn_h);

                        // 最小化
                        let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(rect, 0.0, hover_bg);
                        }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "\u{2013}", egui::FontId::proportional(14.0), text_color);
                        if resp.clicked() { ctx.send_viewport_cmd(ViewportCommand::Minimized(true)); }

                        // 最大化/还原
                        let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(rect, 0.0, hover_bg);
                        }
                        let max_icon = if self.last_maximized { "\u{2750}" } else { "\u{25A1}" };
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, max_icon, egui::FontId::proportional(13.0), text_color);
                        if resp.clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(!self.last_maximized));
                            self.last_maximized = !self.last_maximized;
                        }

                        // 关闭
                        let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                        let (bg, fg) = if resp.hovered() { (close_hover_bg, egui::Color32::WHITE) } else { (egui::Color32::TRANSPARENT, text_color) };
                        if bg != egui::Color32::TRANSPARENT { ui.painter().rect_filled(rect, 0.0, bg); }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "\u{2715}", egui::FontId::proportional(13.0), fg);
                        if resp.clicked() { ctx.send_viewport_cmd(ViewportCommand::Close); }
                    });
                });
            });
    }
}

// ==================== 左侧面板 ====================

impl QTermApp {
    /// 渲染左侧面板内容
    fn render_left_pane(&mut self, ui: &mut egui::Ui) {
        let text_color = self.theme.system.text_color;
        let fs = self.preferences.general_font_size;

        egui::Frame::none()
            .show(ui, |ui| {
                ui.set_min_width(LEFT_PANE_WIDTH);
                ui.set_max_width(LEFT_PANE_WIDTH);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        self.sidebar_label(ui, "终端", fs, text_color);
                    });
                    ui.add_space(4.0);
                    ui.separator();

                    self.render_terminal_pane(ui);

                    // 底部工具栏
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("+ SSH").clicked() {
                                self.ssh_dialog.open = true;
                            }
                            if ui.button("+ 本地").clicked() {
                                self.new_tab();
                            }
                            // 主题切换按钮
                            let is_dark = self.theme.is_dark();
                            let theme_label = if is_dark { "浅色" } else { "深色" };
                            if ui.button(theme_label).clicked() {
                                self.theme.toggle_mode();
                                self.theme.system.apply_to_egui(ui.ctx(), self.theme.is_dark(), fs);
                            }
                        });
                        ui.add_space(2.0);
                    });
                });
            });
    }

    /// 渲染终端连接面板
    fn render_terminal_pane(&mut self, ui: &mut egui::Ui) {
        let side_text = self.theme.system.app_side_text_color;
        let text_color = self.theme.system.text_color;
        let active_bg = self.theme.system.app_left_list_bg_color_active;
        let active_fg = self.theme.system.app_left_list_text_color_active;
        let hover_bg = self.theme.system.app_left_list_bg_color_hover;
        let fs = self.preferences.general_font_size;

        let mut open_conn_idx: Option<usize> = None;
        let mut toggle_group: Option<String> = None;

        // 使用 ScrollArea 包裹连接列表，支持滚动
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
            ui.add_space(8.0);

            if !self.connections.is_empty() {
                self.sidebar_label(ui, "连接", fs - 1.0, side_text);
                ui.add_space(4.0);

                let mut current_group = "";
                for (idx, conn) in self.connections.iter().enumerate() {
                    if conn.group_name != current_group {
                        current_group = &conn.group_name;
                        let collapsed = self.collapsed_groups.contains(&conn.group_name);
                        let arrow = if collapsed { "\u{25B6}" } else { "\u{25BC}" };

                        ui.add_space(4.0);
                        let grp_resp = egui::Frame::none()
                            .rounding(egui::Rounding::same(3.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    self.sidebar_label(ui, arrow, fs - 2.0, side_text);
                                    self.sidebar_label(ui, &conn.group_name, fs - 2.0, side_text);
                                });
                            });
                        // 双击分组名收缩/展开
                        if grp_resp.response.double_clicked() {
                            toggle_group = Some(conn.group_name.clone());
                        }
                    }

                    // 如果分组收缩，跳过该分组的连接
                    if self.collapsed_groups.contains(&conn.group_name) {
                        continue;
                    }

                    let is_selected = self.selected_connection == Some(idx);

                    // 预分配交互区域以获取悬停状态
                    let item_h = fs * 1.5;
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), item_h),
                        egui::Sense::click(),
                    );
                    let bg = if is_selected {
                        active_bg
                    } else if resp.hovered() {
                        hover_bg
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let fg = if is_selected { active_fg } else { text_color };

                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, 4.0, bg);
                    }
                    ui.painter().text(
                        egui::Pos2::new(rect.min.x + 18.0, rect.min.y),
                        egui::Align2::LEFT_TOP,
                        &conn.name,
                        self.sidebar_font_id(fs),
                        fg,
                    );

                    if resp.clicked() {
                        self.selected_connection = Some(idx);
                    }
                    if resp.double_clicked() {
                        open_conn_idx = Some(idx);
                    }
                }
            } else {
                self.sidebar_label(ui, "未找到连接配置。", fs - 1.0, side_text);
                ui.add_space(4.0);
                self.sidebar_label(ui, "WhaleTerm 配置不可用。", fs - 2.0, side_text);
            }

            ui.add_space(12.0);
            self.sidebar_label(ui, "打开的标签", fs - 1.0, side_text);
            ui.add_space(4.0);
            for (idx, tab) in self.tabs.iter().enumerate() {
                let selected = idx == self.active_tab;
                let label = if tab.alive() { &tab.title } else { "[已关闭]" };
                let (bg, fg) = if selected {
                    (active_bg, active_fg)
                } else {
                    (egui::Color32::TRANSPARENT, text_color)
                };
                let inner = egui::Frame::none()
                    .fill(bg)
                    .rounding(egui::Rounding::same(4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            self.sidebar_label(ui, label, fs, fg);
                        });
                    });
                if inner.response.clicked() {
                    self.active_tab = idx;
                }
            }
        });

        // 处理分组收缩/展开
        if let Some(group) = toggle_group {
            if self.collapsed_groups.contains(&group) {
                self.collapsed_groups.remove(&group);
            } else {
                self.collapsed_groups.insert(group);
            }
        }

        // 双击连接时打开新 SSH 标签页
        if let Some(idx) = open_conn_idx {
            self.open_connection_tab(idx);
        }
    }

    /// 双击连接项时打开新的 SSH 标签页
    fn open_connection_tab(&mut self, conn_idx: usize) {
        use crate::ssh::{SshConfig, SshAuth};

        let conn = &self.connections[conn_idx];
        // 根据认证模型选择认证方式
        let auth = if conn.private_key.is_empty() {
            SshAuth::Password(conn.password.clone())
        } else {
            SshAuth::PrivateKey {
                path: conn.private_key.clone(),
                passphrase: Some(conn.password.clone()),
            }
        };
        let config = SshConfig {
            host: conn.addr.clone(),
            port: conn.port,
            username: conn.username.clone(),
            auth,
            timeout_secs: 10,
        };
        self.new_tab();
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let Err(e) = tab.layout.add_ssh_pane(config, SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines) {
                self.sftp_error = Some(format!("SSH 错误: {}", e));
                self.close_tab(self.active_tab);
            }
        }
    }
}

// ==================== 底部状态栏 ====================

impl QTermApp {
    /// 渲染底部状态栏
    /// 显示：连接状态指示灯、会话名称、连接状态文字、快捷键提示
    fn render_foot_bar(&self, ctx: &egui::Context) {
        let status_bg = self.theme.system.app_status_bar_bg_color;
        let split_color = self.theme.system.app_split_color;
        let status_text = self.theme.system.app_status_bar_text_color;
        let connected_color = self.theme.extra.term_connected_color;
        let height = self.theme.terminal.font_size * 2.0;

        egui::TopBottomPanel::bottom("foot_bar")
            .frame(egui::Frame::none()
                .fill(status_bg)
                .stroke(egui::Stroke::new(1.0, split_color)))
            .exact_height(height)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(9.0);

                    // 连接状态指示灯（绿色=已连接，红色=已断开）
                    let connected = self.tabs.get(self.active_tab).map_or(false, |t| t.alive());
                    let dot_color = if connected {
                        connected_color
                    } else {
                        egui::Color32::from_rgb(200, 60, 60)
                    };
                    let dot_diam = height / 3.0;
                    let dot_radius = height / 6.0;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(dot_diam, dot_diam), egui::Sense::hover());
                    ui.painter().rect_filled(rect, dot_radius, dot_color);

                    ui.add_space(4.0);

                    // 会话名称
                    let session_name = self.tabs.get(self.active_tab).map_or("无会话", |t| &t.title);
                    ui.label(egui::RichText::new(session_name).size(12.0).color(status_text));

                    // 分隔符
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("|").size(12.0).color(status_text));
                    ui.add_space(4.0);
                    // 连接状态文字
                    let extra_info = if connected { "已连接" } else { "已断开" };
                    ui.label(egui::RichText::new(extra_info).size(12.0).color(status_text));

                    // 右侧：快捷键提示
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Ctrl+T 新建 | Ctrl+Shift+N SSH | Ctrl+Shift+F SFTP | Ctrl+B 面板")
                            .size(15.0)
                            .color(status_text));
                        ui.add_space(8.0);
                    });
                });
            });
    }
}

// ==================== SFTP 打开 / 输入处理 ====================

impl QTermApp {
    /// 处理打开 SFTP 操作
    /// 从当前活动 SSH 终端面板创建 SFTP 面板
    fn handle_open_sftp(&mut self) {
        let sftp_result = {
            let tab = match self.tabs.get(self.active_tab) {
                Some(t) => t,
                None => { self.sftp_error = Some("无活动标签页".to_string()); return; }
            };
            let active_idx = tab.layout.active_pane;
            match tab.layout.panes.get(active_idx) {
                Some(pane) => {
                    match &pane.kind {
                        PaneKind::Terminal { backend: PaneBackend::Ssh(ssh), .. } => ssh.open_sftp(),
                        _ => { self.sftp_error = Some("SFTP 需要活动的 SSH 终端面板".to_string()); return; }
                    }
                }
                None => { self.sftp_error = Some("无活动面板".to_string()); return; }
            }
        };

        match sftp_result {
            Ok(sftp) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Err(e) = tab.layout.add_sftp_pane(sftp, SplitDirection::Vertical) {
                        self.sftp_error = Some(format!("添加 SFTP 面板失败: {}", e));
                    }
                }
                self.sftp_error = None;
            }
            Err(e) => { self.sftp_error = Some(format!("SFTP 错误: {}", e)); }
        }
    }

    /// 处理终端区域的鼠标交互
    /// 支持：右键菜单、双击选词、三击选行、拖拽选择
    fn handle_terminal_mouse(&mut self) {
        let pm = match self.pending_mouse.take() {
            Some(pm) => pm,
            None => return,
        };
        let tab = match self.tabs.get_mut(self.active_tab) {
            Some(t) => t,
            None => return,
        };
        let pane = match tab.layout.active_pane_mut() {
            Some(p) => p,
            None => return,
        };
        let terminal = match &mut pane.kind {
            PaneKind::Terminal { terminal, .. } => terminal,
            PaneKind::Sftp { .. } => return,
        };

        let response = pm.response;
        let cell_width = pm.cell_width;
        let cell_height = pm.cell_height;
        let origin = pm.origin;

        // 右键点击：打开上下文菜单
        if response.secondary_clicked() {
            self.context_menu.show = true;
            self.context_menu.pos = response.hover_pos().unwrap_or(response.rect.center());
        }

        // 双击：选中单词
        if response.double_clicked() {
            if let Some(pos) = response.hover_pos() {
                let col = ((pos.x - origin.x) / cell_width).floor() as usize;
                let row = ((pos.y - origin.y) / cell_height).floor() as usize;
                if let Some((sr, sc, er, ec)) = terminal.word_at(row, col) {
                    terminal.selection = Some(crate::terminal::Selection {
                        start_row: sr, start_col: sc, end_row: er, end_col: ec,
                    });
                }
            }
            return;
        }

        // 三击：选中整行
        if response.triple_clicked() {
            if let Some(pos) = response.hover_pos() {
                let row = ((pos.y - origin.y) / cell_height).floor() as usize;
                if let Some((sr, sc, er, ec)) = terminal.line_range(row) {
                    terminal.selection = Some(crate::terminal::Selection {
                        start_row: sr, start_col: sc, end_row: er, end_col: ec,
                    });
                }
            }
            return;
        }

        // 单击：清除选择，开始新的拖拽选择
        if response.clicked() {
            terminal.selection = None;
        }

        // 拖拽选择：更新选择范围
        if response.dragged() && response.is_pointer_button_down_on() {
            if let Some(pos) = response.hover_pos() {
                let col = ((pos.x - origin.x) / cell_width).floor() as usize;
                let row = ((pos.y - origin.y) / cell_height).floor() as usize;
                let col = col.min(terminal.cols().saturating_sub(1));
                let row = row.min(terminal.rows().saturating_sub(1));
                match &mut terminal.selection {
                    Some(sel) => {
                        sel.end_row = row;
                        sel.end_col = col;
                    }
                    None => {
                        terminal.selection = Some(crate::terminal::Selection {
                            start_row: row, start_col: col, end_row: row, end_col: col,
                        });
                    }
                }
            }
        }
    }

    /// 渲染右键上下文菜单
    /// 包含：复制、粘贴、清屏、水平/垂直分屏选项
    fn render_context_menu(&mut self, ctx: &egui::Context) {
        if !self.context_menu.show {
            return;
        }

        let menu_pos = self.context_menu.pos;
        // 检查当前是否有选中文本
        let has_selection = self.tabs.get(self.active_tab).and_then(|t| {
            let idx = t.layout.active_pane;
            t.layout.panes.get(idx).and_then(|p| match &p.kind {
                PaneKind::Terminal { terminal, .. } => terminal.selected_text(),
                _ => None,
            })
        });

        let mut close_menu = false;
        let mut do_copy = false;
        let mut do_paste = false;
        let mut do_clear = false;
        let mut do_split_h = false;
        let mut do_split_v = false;

        egui::Area::new(egui::Id::new("context_menu"))
            .fixed_pos(menu_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let menu_frame = egui::Frame::popup(ui.style());
                menu_frame.show(ui, |ui| {
                    ui.set_min_width(160.0);
                    ui.vertical(|ui| {
                        let text_color = self.theme.system.text_color;
                        // 复制按钮（有选中文本时可用）
                        let copy_label = if has_selection.is_some() { "复制" } else { "复制（无选中）" };
                        if ui.add(egui::Button::new(
                            egui::RichText::new(copy_label).size(13.0).color(text_color),
                        ).frame(false)).clicked() && has_selection.is_some() {
                            do_copy = true;
                            close_menu = true;
                        }
                        // 粘贴按钮
                        if ui.add(egui::Button::new(
                            egui::RichText::new("粘贴").size(13.0).color(text_color),
                        ).frame(false)).clicked() {
                            do_paste = true;
                            close_menu = true;
                        }
                        ui.separator();
                        // 清屏按钮
                        if ui.add(egui::Button::new(
                            egui::RichText::new("清屏").size(13.0).color(text_color),
                        ).frame(false)).clicked() {
                            do_clear = true;
                            close_menu = true;
                        }
                        // 水平分屏按钮
                        if ui.add(egui::Button::new(
                            egui::RichText::new("水平分屏").size(13.0).color(text_color),
                        ).frame(false)).clicked() {
                            do_split_h = true;
                            close_menu = true;
                        }
                        // 垂直分屏按钮
                        if ui.add(egui::Button::new(
                            egui::RichText::new("垂直分屏").size(13.0).color(text_color),
                        ).frame(false)).clicked() {
                            do_split_v = true;
                            close_menu = true;
                        }
                    });
                });
            });

        // 点击菜单外部关闭菜单
        if ctx.input(|i| i.pointer.any_click()) {
            let area_rect = ctx.memory(|m| m.area_rect(egui::Id::new("context_menu")));
            if let Some(rect) = area_rect {
                if !rect.contains(ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO))) {
                    close_menu = true;
                }
            }
        }

        if close_menu {
            self.context_menu.show = false;
        }

        // 执行菜单操作
        if do_copy {
            self.do_copy_selection(ctx);
        }
        if do_paste {
            self.do_paste(ctx);
        }
        if do_clear {
            self.do_clear_screen();
        }
        if do_split_h {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                let shell = if self.config.shell_path.is_empty() { None } else { Some(self.config.shell_path.as_str()) };
                let _ = tab.layout.add_local_pane(SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines, shell);
            }
        }
        if do_split_v {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                let shell = if self.config.shell_path.is_empty() { None } else { Some(self.config.shell_path.as_str()) };
                let _ = tab.layout.add_local_pane(SplitDirection::Vertical, self.last_rows, self.last_cols, self.config.scrollback_lines, shell);
            }
        }
    }

    /// 复制选中文本到剪贴板
    fn do_copy_selection(&mut self, ctx: &egui::Context) {
        let text = self.tabs.get(self.active_tab).and_then(|t| {
            let idx = t.layout.active_pane;
            t.layout.panes.get(idx).and_then(|p| match &p.kind {
                PaneKind::Terminal { terminal, .. } => terminal.selected_text(),
                _ => None,
            })
        });
        if let Some(text) = text {
            ctx.output_mut(|o| o.copied_text = text);
        }
    }

    /// 粘贴：请求系统剪贴板内容
    fn do_paste(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }

    /// 清屏：清除终端内容并发送清屏指令到后端
    fn do_clear_screen(&mut self) {
        let tab = match self.tabs.get_mut(self.active_tab) {
            Some(t) => t,
            None => return,
        };
        let pane = match tab.layout.active_pane_mut() {
            Some(p) => p,
            None => return,
        };
        match &mut pane.kind {
            PaneKind::Terminal { terminal, backend } => {
                // 清除终端网格内容
                for row in 0..terminal.rows() {
                    terminal.grid.clear_row(row);
                }
                terminal.cursor.row = 0;
                terminal.cursor.col = 0;
                // 发送 ANSI 清屏指令到 PTY/SSH
                match backend {
                    PaneBackend::Local(pty) => { let _ = pty.write(b"\x1b[2J\x1b[H"); }
                    PaneBackend::Ssh(ssh) => { let _ = ssh.write(b"\x1b[2J\x1b[H"); }
                }
            }
            PaneKind::Sftp { .. } => {}
        }
    }

    /// 处理用户输入事件
    /// 包括鼠标事件、键盘事件（文本输入、快捷键、特殊键）
    fn handle_input(&mut self, ctx: &egui::Context) {
        // 处理终端鼠标交互
        self.handle_terminal_mouse();

        // 渲染右键上下文菜单
        self.render_context_menu(ctx);

        let tab = match self.tabs.get_mut(self.active_tab) {
            Some(t) => t,
            None => return,
        };
        let pane = match tab.layout.active_pane_mut() {
            Some(p) => p,
            None => return,
        };

        match &mut pane.kind {
            PaneKind::Terminal { terminal, backend } => {
                // 终端面板获得焦点时取消 UI 元素焦点
                if ctx.memory(|m| m.focused().is_some()) {
                    ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap()));
                }

                ctx.input(|i| {
                    for event in &i.events {
                        match event {
                            egui::Event::Text(text) => {
                                // Ctrl 键按下时不处理文本输入（由快捷键处理）
                                if i.modifiers.ctrl || i.modifiers.command {
                                    continue;
                                }
                                // 将文本写入 PTY/SSH
                                match backend {
                                    PaneBackend::Local(pty) => { let _ = pty.write(text.as_bytes()); }
                                    PaneBackend::Ssh(ssh) => { let _ = ssh.write(text.as_bytes()); }
                                }
                            }
                            egui::Event::Paste(text) => {
                                // 粘贴文本到 PTY/SSH
                                if !text.is_empty() {
                                    match backend {
                                        PaneBackend::Local(pty) => { let _ = pty.write(text.as_bytes()); }
                                        PaneBackend::Ssh(ssh) => { let _ = ssh.write(text.as_bytes()); }
                                    }
                                }
                            }
                            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                // Ctrl+C：复制选中文本或发送 SIGINT（\x03）
                                if *key == egui::Key::C && modifiers.ctrl && !modifiers.shift {
                                    if let Some(text) = terminal.selected_text() {
                                        ctx.output_mut(|o| o.copied_text = text);
                                        terminal.selection = None;
                                    } else {
                                        match backend {
                                            PaneBackend::Local(pty) => { let _ = pty.write(b"\x03"); }
                                            PaneBackend::Ssh(ssh) => { let _ = ssh.write(b"\x03"); }
                                        }
                                    }
                                    continue;
                                }
                                // Ctrl+Shift+C：强制复制选中文本
                                if *key == egui::Key::C && modifiers.ctrl && modifiers.shift {
                                    if let Some(text) = terminal.selected_text() {
                                        ctx.output_mut(|o| o.copied_text = text);
                                    }
                                    continue;
                                }
                                // Ctrl+V / Ctrl+Shift+V：请求粘贴
                                if *key == egui::Key::V && modifiers.ctrl {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                                    continue;
                                }
                                // 将按键转换为 ANSI 转义序列写入后端
                                if let Some(seq) = key_to_seq(*key, *modifiers) {
                                    match backend {
                                        PaneBackend::Local(pty) => { let _ = pty.write(seq.as_bytes()); }
                                        PaneBackend::Ssh(ssh) => { let _ = ssh.write(seq.as_bytes()); }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                });

                // 发送终端待回复的 ANSI 响应
                for reply in terminal.pending_replies.drain(..) {
                    match backend {
                        PaneBackend::Local(pty) => { let _ = pty.write(&reply); }
                        PaneBackend::Ssh(ssh) => { let _ = ssh.write(&reply); }
                    }
                }
            }
            PaneKind::Sftp { .. } => {}
        }
    }
}

/// 将按键和修饰键转换为 ANSI 转义序列
/// 支持 Ctrl+字母、方向键、功能键等
fn key_to_seq(key: egui::Key, mods: egui::Modifiers) -> Option<String> {
    // Ctrl + 字母键 → 控制字符
    if mods.ctrl && !mods.shift {
        let ctrl_char = match key {
            egui::Key::A => Some("\x01"), egui::Key::B => Some("\x02"),
            egui::Key::C => Some("\x03"), egui::Key::D => Some("\x04"),
            egui::Key::E => Some("\x05"), egui::Key::F => Some("\x06"),
            egui::Key::G => Some("\x07"), egui::Key::H => Some("\x08"),
            egui::Key::K => Some("\x0B"), egui::Key::L => Some("\x0C"),
            egui::Key::N => Some("\x0E"), egui::Key::O => Some("\x0F"),
            egui::Key::P => Some("\x10"), egui::Key::Q => Some("\x11"),
            egui::Key::R => Some("\x12"), egui::Key::S => Some("\x13"),
            egui::Key::U => Some("\x15"), egui::Key::Z => Some("\x1A"),
            _ => None,
        };
        if let Some(s) = ctrl_char { return Some(s.to_string()); }
    }

    // 特殊键 → ANSI 转义序列
    match key {
        egui::Key::Enter => Some("\r".to_string()),
        egui::Key::Backspace => Some("\x7f".to_string()),
        egui::Key::Tab => Some("\t".to_string()),
        egui::Key::Escape => Some("\x1b".to_string()),
        egui::Key::ArrowUp => Some("\x1b[A".to_string()),
        egui::Key::ArrowDown => Some("\x1b[B".to_string()),
        egui::Key::ArrowRight => Some("\x1b[C".to_string()),
        egui::Key::ArrowLeft => Some("\x1b[D".to_string()),
        egui::Key::Home => Some("\x1b[H".to_string()),
        egui::Key::End => Some("\x1b[F".to_string()),
        egui::Key::PageUp => Some("\x1b[5~".to_string()),
        egui::Key::PageDown => Some("\x1b[6~".to_string()),
        egui::Key::Delete => Some("\x1b[3~".to_string()),
        egui::Key::Insert => Some("\x1b[2~".to_string()),
        _ => None,
    }
}