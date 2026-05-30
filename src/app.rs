use eframe::egui;

use crate::config::AppConfig;
use crate::tab::Tab;
use crate::terminal::renderer;
use crate::theme::AppTheme;
use crate::ui::split_pane::{SplitDirection, PaneKind, PaneBackend};

const TITLE_BAR_HEIGHT: f32 = 40.0;
const RIBBON_WIDTH: f32 = 50.0;
const LEFT_PANE_WIDTH: f32 = 220.0;

pub struct QTermApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    config: AppConfig,
    theme: AppTheme,
    last_window_pos: Option<(f32, f32)>,
    last_window_size: Option<(f32, f32)>,
    last_maximized: bool,
    last_cols: usize,
    last_rows: usize,
    ssh_dialog: crate::ui::ssh_dialog::SshDialog,
    sftp_error: Option<String>,
    show_left_pane: bool,
    ribbon_active: RibbonSection,
}

#[derive(Clone, Copy, PartialEq)]
enum RibbonSection {
    Terminal,
    Sftp,
    Settings,
}

impl QTermApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        Self::configure_fonts(&cc.egui_ctx);

        let is_dark = config.theme != "light";
        let theme = if is_dark { AppTheme::dark() } else { AppTheme::light() };
        theme.system.apply_to_egui(&cc.egui_ctx, is_dark);
        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            config,
            theme,
            last_window_pos: None,
            last_window_size: None,
            last_maximized: false,
            last_cols: 80,
            last_rows: 24,
            ssh_dialog: crate::ui::ssh_dialog::SshDialog::new(),
            sftp_error: None,
            show_left_pane: true,
            ribbon_active: RibbonSection::Terminal,
        };
        app.new_tab();
        app
    }

    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let font_paths: Vec<&str> = if cfg!(target_os = "windows") {
            vec![
                "C:\\Windows\\Fonts\\msyh.ttc",
                "C:\\Windows\\Fonts\\consola.ttf",
            ]
        } else if cfg!(target_os = "macos") {
            vec!["/System/Library/Fonts/PingFang.ttc"]
        } else {
            vec!["/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"]
        };

        for path in font_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    path.to_string(),
                    egui::FontData::from_owned(data).into(),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push(path.to_string());
            }
        }
        ctx.set_fonts(fonts);
    }

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
                eprintln!("Failed to create tab: {}", e);
            }
        }
    }

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

impl eframe::App for QTermApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for tab in &mut self.tabs {
            tab.poll();
        }

        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.last_window_size = Some((rect.width(), rect.height()));
            }
            if let Some(pos) = i.viewport().outer_rect {
                self.last_window_pos = Some((pos.min.x, pos.min.y));
            }
        });

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
        });
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
            None => {}
        }

        // === Title Bar (40px) ===
        self.render_title_bar(ctx);

        // === Left SidePanel: Ribbon + LeftPane ===
        let left_total = if self.show_left_pane {
            RIBBON_WIDTH + LEFT_PANE_WIDTH
        } else {
            RIBBON_WIDTH
        };
        egui::SidePanel::left("left_panel")
            .frame(egui::Frame::none())
            .exact_width(left_total)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    self.render_ribbon(ui);
                    if self.show_left_pane {
                        ui.separator();
                        self.render_left_pane(ui);
                    }
                });
            });

        // === FootBar (status bar) ===
        self.render_foot_bar(ctx);

        // === Central panel: terminal ===
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.system.app_content_term_bg_color))
            .show(ctx, |ui| {
                if self.tabs.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("Press Ctrl+T to open a new terminal");
                    });
                    return;
                }

                let pane_count = {
                    let tab = &self.tabs[self.active_tab];
                    tab.layout.pane_count()
                };

                let size = renderer::calculate_size(ui, self.theme.terminal.font_size);
                let (target_rows, target_cols) = if pane_count <= 1 {
                    (size.rows, size.cols)
                } else {
                    let tab = &self.tabs[self.active_tab];
                    match tab.layout.direction {
                        SplitDirection::Horizontal => ((size.rows / pane_count).max(1), size.cols),
                        SplitDirection::Vertical => (size.rows, (size.cols / pane_count).max(1)),
                    }
                };
                if target_rows != self.last_rows || target_cols != self.last_cols {
                    self.last_rows = target_rows;
                    self.last_cols = target_cols;
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        for pane in &mut tab.layout.panes {
                            pane.resize(target_rows, target_cols);
                        }
                    }
                }

                let tab = &mut self.tabs[self.active_tab];
                if pane_count <= 1 {
                    if let Some(pane) = tab.layout.active_pane_mut() {
                        match &mut pane.kind {
                            PaneKind::Terminal { terminal, .. } => {
                                renderer::render(ui, terminal, &self.theme.terminal);
                            }
                            PaneKind::Sftp { panel } => {
                                panel.show(ui);
                            }
                        }
                    }
                } else {
                    let active_idx = tab.layout.active_pane;
                    match tab.layout.direction {
                        SplitDirection::Horizontal => {
                            let available_height = ui.available_height();
                            let pane_height = available_height / pane_count as f32;
                            for (idx, pane) in tab.layout.panes.iter_mut().enumerate() {
                                let is_active = idx == active_idx;
                                let stroke = if is_active {
                                    egui::Stroke::new(1.0, self.theme.system.text_active_color)
                                } else {
                                    egui::Stroke::NONE
                                };
                                egui::Frame::none()
                                    .fill(self.theme.system.app_content_term_bg_color)
                                    .stroke(stroke)
                                    .show(ui, |ui| {
                                        ui.set_max_height(pane_height - 2.0);
                                        match &mut pane.kind {
                                            PaneKind::Terminal { terminal, .. } => {
                                                renderer::render(ui, terminal, &self.theme.terminal);
                                            }
                                            PaneKind::Sftp { panel } => {
                                                panel.show(ui);
                                            }
                                        }
                                    });
                            }
                        }
                        SplitDirection::Vertical => {
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
                                        .fill(self.theme.system.app_content_term_bg_color)
                                        .stroke(stroke)
                                        .show(ui, |ui| {
                                            ui.set_max_width(pane_width - 2.0);
                                            match &mut pane.kind {
                                                PaneKind::Terminal { terminal, .. } => {
                                                    renderer::render(ui, terminal, &self.theme.terminal);
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

        self.ssh_dialog.show(ctx);

        if let Some(config) = self.ssh_dialog.result.take() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                if let Err(e) = tab.layout.add_ssh_pane(config, SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines) {
                    self.ssh_dialog.status = Some(format!("SSH error: {}", e));
                    self.ssh_dialog.open = true;
                }
            }
        }

        self.handle_input(ctx);
        ctx.request_repaint();
    }

    fn on_exit(&mut self) {
        self.config.window_x = self.last_window_pos.map(|(x, _)| x);
        self.config.window_y = self.last_window_pos.map(|(_, y)| y);
        self.config.window_width = self.last_window_size.map(|(w, _)| w);
        self.config.window_height = self.last_window_size.map(|(_, h)| h);
        self.config.theme = if self.theme.is_dark() { "dark".to_string() } else { "light".to_string() };
        self.config.save();
        for tab in &mut self.tabs {
            tab.close();
        }
    }
}

enum Action {
    NewTab,
    CloseTab,
    NextTab,
    SplitHorizontal,
    SplitVertical,
    NextPane,
    ClosePane,
    OpenSshDialog,
    OpenSftp,
    ToggleLeftPane,
}

// === Title Bar ===

impl QTermApp {
    fn render_title_bar(&mut self, ctx: &egui::Context) {
        let sys = &self.theme.system;
        let title_bar_h = TITLE_BAR_HEIGHT;

        egui::TopBottomPanel::top("title_bar")
            .frame(egui::Frame::none().fill(sys.app_bg_color))
            .exact_height(title_bar_h)
            .show(ctx, |ui| {
                // Title bar is draggable for window movement
                let title_bar_response = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("title_bar_drag"),
                    egui::Sense::click_and_drag(),
                );
                if title_bar_response.double_clicked() {
                    // Toggle maximize
                    let _ = ctx.send_viewport_cmd(egui::ViewportCommand::ToggleMaximized);
                }
                if title_bar_response.dragged() {
                    let _ = ctx.send_viewport_cmd(egui::ViewportCommand::StartInnerDrag);
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // Logo + title
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("QTerm").font(egui::FontId::proportional(16.0)).strong().color(sys.app_header_text_color));
                    ui.add_space(16.0);

                    // Card-style tabs
                    let mut close_idx = None;
                    let tab_h = title_bar_h - 6.0;
                    for (idx, tab) in self.tabs.iter().enumerate() {
                        let selected = idx == self.active_tab;
                        let label = if tab.alive() { &tab.title } else { "[closed]" };

                        let (text_color, bg_color) = if selected {
                            (sys.text_active_color, sys.app_side_hover_bg_color)
                        } else {
                            (sys.app_side_text_color, egui::Color32::TRANSPARENT)
                        };

                        let tab_resp = egui::Frame::none()
                            .fill(bg_color)
                            .rounding(egui::Rounding::nw_ne(8.0))
                            .show(ui, |ui| {
                                ui.set_min_height(tab_h);
                                ui.set_max_height(tab_h);
                                ui.horizontal_centered(|ui| {
                                    ui.add_space(8.0);
                                    if ui.label(egui::RichText::new(label).color(text_color).size(13.0)).clicked() {
                                        // tab click handled below
                                    }
                                    ui.add_space(4.0);
                                    let close_btn = ui.add_s(egui::Button::new(
                                        egui::RichText::new("x").size(11.0).color(text_color),
                                    ).frame(false).desired_height(16.0).desired_width(20.0));
                                    if close_btn.clicked() {
                                        close_idx = Some(idx);
                                    }
                                    ui.add_space(4.0);
                                })
                            })
                            .response;

                        if tab_resp.clicked() {
                            self.active_tab = idx;
                        }
                        ui.add_space(1.0);
                    }

                    // "+" button
                    let plus_resp = ui.add_s(egui::Button::new(
                        egui::RichText::new("+").size(16.0).color(sys.app_side_text_color),
                    ).frame(false));
                    if plus_resp.clicked() {
                        self.new_tab();
                    }

                    if let Some(idx) = close_idx {
                        self.close_tab(idx);
                    }

                    // Push window controls to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_size = egui::vec2(46.0, title_bar_h);
                        // Close button
                        let close_hover = ui.rect_contains_pointer(ui.max_rect());
                        let close_color = if close_hover {
                            egui::Color32::from_rgb(196, 43, 28)
                        } else {
                            sys.text_color
                        };
                        if ui.put(egui::Rect::from_min_size(
                            ui.max_rect().right_top() - egui::vec2(btn_size.x, 0.0),
                            btn_size,
                        ), egui::Button::new(
                            egui::RichText::new("x").size(13.0).color(close_color),
                        ).frame(false).desired_size(btn_size)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.add_space(46.0); // reserve space for close
                        // Maximize button
                        if ui.add_s(egui::Button::new(
                            egui::RichText::new("O").size(13.0).color(sys.text_color),
                        ).frame(false).desired_size(btn_size)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::ToggleMaximized);
                        }
                        // Minimize button
                        if ui.add_s(egui::Button::new(
                            egui::RichText::new("-").size(16.0).color(sys.text_color),
                        ).frame(false).desired_size(btn_size)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimize);
                        }
                    });
                });
            });
    }
}

// === Ribbon (left icon bar) ===

impl QTermApp {
    fn render_ribbon(&mut self, ui: &mut egui::Ui) {
        let sys = &self.theme.system;
        let icon_size = RIBBON_WIDTH - 10.0;

        egui::Frame::none()
            .fill(sys.app_sider_bar_bg_color)
            .stroke(egui::Stroke::new(1.0, sys.app_split_color))
            .show(ui, |ui| {
                ui.set_min_width(RIBBON_WIDTH);
                ui.set_max_width(RIBBON_WIDTH);
                ui.vertical(|ui| {
                    ui.add_space(5.0);

                    // Terminal section button
                    let terminal_active = self.ribbon_active == RibbonSection::Terminal;
                    let terminal_resp = ribbon_button(ui, ">_", icon_size, terminal_active, sys);
                    if terminal_resp.clicked() {
                        self.ribbon_active = RibbonSection::Terminal;
                    }

                    // SFTP section button
                    let sftp_active = self.ribbon_active == RibbonSection::Sftp;
                    let sftp_resp = ribbon_button(ui, "F", icon_size, sftp_active, sys);
                    if sftp_resp.clicked() {
                        self.ribbon_active = RibbonSection::Sftp;
                    }

                    // Spacer pushes bottom buttons down
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        // Theme toggle
                        let theme_icon = if self.theme.is_dark() { "L" } else { "D" };
                        if ui.add_s(egui::Button::new(
                            egui::RichText::new(theme_icon).size(14.0).color(sys.app_side_text_color),
                        ).frame(false).desired_size(egui::vec2(30.0, 30.0))
                         .rounding(egui::Rounding::same(4.0))).clicked() {
                            self.theme.toggle_mode();
                            self.theme.system.apply_to_egui(ui.ctx(), self.theme.is_dark());
                        }
                        ui.add_space(2.0);

                        // Settings button
                        let settings_resp = ribbon_button(ui, "*", 30.0, false, sys);
                        let _ = settings_resp;
                        ui.add_space(5.0);
                    });
                });
            });
    }
}

fn ribbon_button(ui: &mut egui::Ui, icon: &str, size: f32, active: bool, sys: &crate::theme::system::SystemTheme) -> egui::Response {
    let (bg, fg) = if active {
        (sys.app_side_hover_bg_color, sys.app_side_text_active_color)
    } else {
        (egui::Color32::TRANSPARENT, sys.app_side_text_color)
    };
    let resp = ui.add_s(
        egui::Button::new(
            egui::RichText::new(icon).size(size * 0.4).color(fg).strong(),
        )
        .frame(false)
        .desired_size(egui::vec2(size, size))
        .rounding(egui::Rounding::same(8.0))
        .fill(bg),
    );
    resp
}

// === Left Pane ===

impl QTermApp {
    fn render_left_pane(&mut self, ui: &mut egui::Ui) {
        let sys = &self.theme.system;

        egui::Frame::none()
            .fill(sys.app_left_list_bg_color)
            .show(ui, |ui| {
                ui.set_min_width(LEFT_PANE_WIDTH);
                ui.set_max_width(LEFT_PANE_WIDTH);
                ui.vertical(|ui| {
                    // Header
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let title = match self.ribbon_active {
                            RibbonSection::Terminal => "Terminal",
                            RibbonSection::Sftp => "SFTP",
                            RibbonSection::Settings => "Settings",
                        };
                        ui.label(egui::RichText::new(title).strong().color(sys.text_color).size(14.0));
                    });
                    ui.add_space(4.0);
                    ui.separator();

                    match self.ribbon_active {
                        RibbonSection::Terminal => self.render_terminal_pane(ui),
                        RibbonSection::Sftp => self.render_sftp_section(ui),
                        RibbonSection::Settings => self.render_settings_section(ui),
                    }

                    // Bottom toolbar
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Left), |ui| {
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("+ New").clicked() {
                                self.ssh_dialog.open = true;
                            }
                            if ui.button("Local").clicked() {
                                self.new_tab();
                            }
                        });
                        ui.add_space(2.0);
                    });
                });
            });
    }

    fn render_terminal_pane(&mut self, ui: &mut egui::Ui) {
        let sys = &self.theme.system;
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Quick Connect").size(12.0).color(sys.app_side_text_color));
            ui.add_space(4.0);
            if ui.button("SSH Connection...").clicked() {
                self.ssh_dialog.open = true;
            }
            ui.add_space(12.0);
            ui.label(egui::RichText::new("Open Tabs").size(12.0).color(sys.app_side_text_color));
            ui.add_space(4.0);
            for (idx, tab) in self.tabs.iter().enumerate() {
                let selected = idx == self.active_tab;
                let label = if tab.alive() { &tab.title } else { "[closed]" };
                let (bg, fg) = if selected {
                    (sys.app_left_list_bg_color_active, sys.app_left_list_text_color_active)
                } else {
                    (egui::Color32::TRANSPARENT, sys.text_color)
                };
                let resp = egui::Frame::none()
                    .fill(bg)
                    .rounding(egui::Rounding::same(4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(label).size(13.0).color(fg));
                        })
                    })
                    .response;
                if resp.clicked() {
                    self.active_tab = idx;
                }
            }
        });
    }

    fn render_sftp_section(&mut self, ui: &mut egui::Ui) {
        let sys = &self.theme.system;
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("SFTP requires an SSH connection.").size(12.0).color(sys.app_side_text_color));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Use Ctrl+Shift+F in an SSH tab to open SFTP.").size(11.0).color(sys.app_side_text_color));
        });
    }

    fn render_settings_section(&mut self, ui: &mut egui::Ui) {
        let sys = &self.theme.system;
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Theme").size(12.0).color(sys.app_side_text_color));
            ui.add_space(4.0);
            let is_dark = self.theme.is_dark();
            let label = if is_dark { "Switch to Light" } else { "Switch to Dark" };
            if ui.button(label).clicked() {
                self.theme.toggle_mode();
                self.theme.system.apply_to_egui(ui.ctx(), self.theme.is_dark());
            }
        });
    }
}

// === FootBar (status bar) ===

impl QTermApp {
    fn render_foot_bar(&self, ctx: &egui::Context) {
        let sys = &self.theme.system;
        let height = self.theme.terminal.font_size * 2.0;

        egui::TopBottomPanel::bottom("foot_bar")
            .frame(egui::Frame::none()
                .fill(sys.app_status_bar_bg_color)
                .stroke(egui::Stroke::new(1.0, sys.app_split_color)))
            .exact_height(height)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(9.0);

                    // Connection status dot
                    let connected = self.tabs.get(self.active_tab).map_or(false, |t| t.alive());
                    let dot_color = if connected {
                        self.theme.extra.term_connected_color
                    } else {
                        egui::Color32::from_rgb(200, 60, 60)
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, dot_color);

                    ui.add_space(4.0);

                    // Session name
                    let session_name = self.tabs.get(self.active_tab).map_or("No session", |t| &t.title);
                    ui.label(egui::RichText::new(session_name).size(12.0).color(sys.app_status_bar_text_color));

                    // Right side: push buttons to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        // Keyboard shortcut hints
                        ui.label(egui::RichText::new("Ctrl+T New | Ctrl+Shift+N SSH | Ctrl+Shift+F SFTP | Ctrl+B Panel")
                            .size(11.0)
                            .color(egui::Color32::from_rgba_premultiplied(150, 150, 150, 180)));
                        ui.add_space(8.0);
                    });
                });
            });
    }
}

// === SFTP open / Input handling (unchanged logic) ===

impl QTermApp {
    fn handle_open_sftp(&mut self) {
        let sftp_result = {
            let tab = match self.tabs.get(self.active_tab) {
                Some(t) => t,
                None => { self.sftp_error = Some("No active tab".to_string()); return; }
            };
            let active_idx = tab.layout.active_pane;
            match tab.layout.panes.get(active_idx) {
                Some(pane) => {
                    match &pane.kind {
                        PaneKind::Terminal { backend: PaneBackend::Ssh(ssh), .. } => ssh.open_sftp(),
                        _ => { self.sftp_error = Some("SFTP requires an active SSH terminal pane".to_string()); return; }
                    }
                }
                None => { self.sftp_error = Some("No active pane".to_string()); return; }
            }
        };

        match sftp_result {
            Ok(sftp) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Err(e) = tab.layout.add_sftp_pane(sftp, SplitDirection::Vertical) {
                        self.sftp_error = Some(format!("Failed to add SFTP pane: {}", e));
                    }
                }
                self.sftp_error = None;
            }
            Err(e) => { self.sftp_error = Some(format!("SFTP error: {}", e)); }
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
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
                if ctx.memory(|m| m.focused().is_some()) {
                    ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap()));
                }

                ctx.input(|i| {
                    for event in &i.events {
                        match event {
                            egui::Event::Text(text) => {
                                match backend {
                                    PaneBackend::Local(pty) => { let _ = pty.write(text.as_bytes()); }
                                    PaneBackend::Ssh(ssh) => { let _ = ssh.write(text.as_bytes()); }
                                }
                            }
                            egui::Event::Key { key, pressed: true, modifiers, .. } => {
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

fn key_to_seq(key: egui::Key, mods: egui::Modifiers) -> Option<String> {
    if mods.ctrl {
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
