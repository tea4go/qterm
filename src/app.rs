use eframe::egui;

use crate::config::AppConfig;
use crate::tab::Tab;
use crate::terminal::renderer;
use crate::theme::AppTheme;
use crate::ui::split_pane::{SplitDirection, PaneKind, PaneBackend};


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
        // Poll all tabs for PTY output
        for tab in &mut self.tabs {
            tab.poll();
        }

        // Track window state for config persistence
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.last_window_size = Some((rect.width(), rect.height()));
            }
            if let Some(pos) = i.viewport().outer_rect {
                self.last_window_pos = Some((pos.min.x, pos.min.y));
            }
        });

        // Handle global shortcuts
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
            if i.key_pressed(egui::Key::T)
                && i.modifiers.ctrl
                && !i.modifiers.shift
            {
                action = Some(Action::NewTab);
            }
            if i.key_pressed(egui::Key::W) && i.modifiers.ctrl && !i.modifiers.shift {
                action = Some(Action::CloseTab);
            }
            if i.key_pressed(egui::Key::Tab) && i.modifiers.ctrl {
                action = Some(Action::NextTab);
            }
        });
        match action {
            Some(Action::NewTab) => self.new_tab(),
            Some(Action::CloseTab) => {
                let idx = self.active_tab;
                self.close_tab(idx);
            }
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
            Some(Action::OpenSshDialog) => {
                self.ssh_dialog.open = true;
            }
            Some(Action::OpenSftp) => {
                self.handle_open_sftp();
            }
            None => {}
        }

        // Tab bar
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame::none()
                .fill(self.theme.system.app_bg_color)
                .stroke(egui::Stroke::new(1.0, self.theme.system.app_split_color)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut close_idx = None;
                    for (idx, tab) in self.tabs.iter().enumerate() {
                        let selected = idx == self.active_tab;
                        let label = if tab.alive() { &tab.title } else { "[closed]" };
                        if ui.selectable_label(selected, label).clicked() {
                            self.active_tab = idx;
                        }
                        if ui.small_button("x").clicked() {
                            close_idx = Some(idx);
                        }
                        ui.separator();
                    }
                    if ui.button("+").clicked() {
                        self.new_tab();
                    }
                    if let Some(idx) = close_idx {
                        self.close_tab(idx);
                    }
                });
            });

        // Central panel: terminal
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

                // Calculate and apply terminal resize
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
                    // Single pane: full screen render
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
                    // Multi-pane split rendering
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

        // Show SSH dialog
        self.ssh_dialog.show(ctx);

        // Handle SSH dialog result
        if let Some(config) = self.ssh_dialog.result.take() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                if let Err(e) = tab.layout.add_ssh_pane(config, SplitDirection::Horizontal, self.last_rows, self.last_cols, self.config.scrollback_lines) {
                    self.ssh_dialog.status = Some(format!("SSH error: {}", e));
                    self.ssh_dialog.open = true;
                }
            }
        }

        // Handle keyboard input via raw events
        self.handle_input(ctx);

        // Keep repainting for terminal updates
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
}

impl QTermApp {
    fn handle_open_sftp(&mut self) {
        let sftp_result = {
            let tab = match self.tabs.get(self.active_tab) {
                Some(t) => t,
                None => {
                    self.sftp_error = Some("No active tab".to_string());
                    return;
                }
            };
            let active_idx = tab.layout.active_pane;
            match tab.layout.panes.get(active_idx) {
                Some(pane) => {
                    match &pane.kind {
                        PaneKind::Terminal { backend: PaneBackend::Ssh(ssh), .. } => {
                            ssh.open_sftp()
                        }
                        _ => {
                            self.sftp_error = Some("SFTP requires an active SSH terminal pane".to_string());
                            return;
                        }
                    }
                }
                None => {
                    self.sftp_error = Some("No active pane".to_string());
                    return;
                }
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
            Err(e) => {
                self.sftp_error = Some(format!("SFTP error: {}", e));
            }
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

        // Only send keyboard input to terminal panes
        match &mut pane.kind {
            PaneKind::Terminal { terminal, backend } => {
                // Ensure no widget steals keyboard focus from the terminal
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
                            egui::Event::Key {
                                key,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
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
            egui::Key::A => Some("\x01"),
            egui::Key::B => Some("\x02"),
            egui::Key::C => Some("\x03"),
            egui::Key::D => Some("\x04"),
            egui::Key::E => Some("\x05"),
            egui::Key::F => Some("\x06"),
            egui::Key::G => Some("\x07"),
            egui::Key::H => Some("\x08"),
            egui::Key::K => Some("\x0B"),
            egui::Key::L => Some("\x0C"),
            egui::Key::N => Some("\x0E"),
            egui::Key::O => Some("\x0F"),
            egui::Key::P => Some("\x10"),
            egui::Key::Q => Some("\x11"),
            egui::Key::R => Some("\x12"),
            egui::Key::S => Some("\x13"),
            egui::Key::U => Some("\x15"),
            egui::Key::Z => Some("\x1A"),
            _ => None,
        };
        if let Some(s) = ctrl_char {
            return Some(s.to_string());
        }
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
