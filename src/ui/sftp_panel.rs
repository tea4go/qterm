use eframe::egui;

use crate::sftp::{self, SftpEvent, FileEntry};

struct LocalFileEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

pub struct SftpPanel {
    sftp: sftp::SftpHandle,
    local_path: String,
    remote_path: String,
    local_entries: Vec<LocalFileEntry>,
    remote_entries: Vec<FileEntry>,
    selected_local: Option<usize>,
    selected_remote: Option<usize>,
    status: String,
    connected: bool,
    pending_list: bool,
}

impl SftpPanel {
    pub fn new(sftp: sftp::SftpHandle) -> Self {
        let local_path = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());

        let mut panel = Self {
            sftp,
            local_path,
            remote_path: "/".to_string(),
            local_entries: Vec::new(),
            remote_entries: Vec::new(),
            selected_local: None,
            selected_remote: None,
            status: "Connecting...".to_string(),
            connected: false,
            pending_list: false,
        };
        panel.refresh_local();
        panel
    }

    pub fn poll(&mut self) {
        for event in self.sftp.poll() {
            match event {
                SftpEvent::Connected => {
                    self.connected = true;
                    self.status = "Connected".to_string();
                    self.pending_list = true;
                    self.sftp.list_dir(&self.remote_path);
                }
                SftpEvent::DirListing(entries) => {
                    self.remote_entries = entries;
                    self.selected_remote = None;
                    self.pending_list = false;
                }
                SftpEvent::UploadDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "Upload complete".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("Upload failed: {}", e),
                    }
                }
                SftpEvent::DownloadDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "Download complete".to_string();
                            self.refresh_local();
                        }
                        Err(e) => self.status = format!("Download failed: {}", e),
                    }
                }
                SftpEvent::MkdirDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "Directory created".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("Mkdir failed: {}", e),
                    }
                }
                SftpEvent::DeleteDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "Deleted".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("Delete failed: {}", e),
                    }
                }
                SftpEvent::Error(e) => {
                    if self.pending_list {
                        self.pending_list = false;
                    }
                    self.status = e;
                }
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let available = ui.available_size();
            let half_w = available.x / 2.0;
            let btn_h = 28.0;
            let list_h = (available.y - btn_h).max(100.0);

            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(half_w);
                    ui.set_min_size(egui::vec2(half_w, list_h));
                    self.show_local_pane(ui);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_max_width(half_w);
                    ui.set_min_size(egui::vec2(half_w, list_h));
                    self.show_remote_pane(ui);
                });
            });

            ui.separator();
            ui.horizontal(|ui| {
                let can_upload = self.selected_local.is_some();
                let can_download = self.selected_remote.is_some();

                if ui.add_enabled(can_upload, egui::Button::new("Upload ->")).clicked() {
                    self.do_upload();
                }
                if ui.add_enabled(can_download, egui::Button::new("<- Download")).clicked() {
                    self.do_download();
                }
                ui.separator();
                ui.label(&self.status);
            });
        });
    }

    pub fn is_alive(&self) -> bool {
        self.sftp.is_alive()
    }

    pub fn close(&mut self) {
        self.sftp.disconnect();
    }

    fn show_local_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Local:");
            if ui.button("..").clicked() {
                self.navigate_local_up();
            }
        });
        ui.label(
            egui::RichText::new(&self.local_path).small().color(egui::Color32::from_rgba_premultiplied(180, 180, 180, 200)),
        );

        let mut navigate_idx = None;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for (i, entry) in self.local_entries.iter().enumerate() {
                    let selected = self.selected_local == Some(i);
                    let icon = if entry.is_dir { "[D]" } else { "   " };
                    let label = format!("{} {}  {}", icon, entry.name, format_size(entry.size));

                    let resp = ui.selectable_label(selected, label);
                    if resp.clicked() {
                        self.selected_local = Some(i);
                    }
                    if resp.double_clicked() && entry.is_dir {
                        navigate_idx = Some(i);
                    }
                }
            });
        if let Some(idx) = navigate_idx {
            if let Some(entry) = self.local_entries.get(idx) {
                let name = entry.name.clone();
                self.navigate_local_into(&name);
            }
        }
    }

    fn show_remote_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Remote:");
            if ui.button("..").clicked() {
                self.navigate_remote_up();
            }
        });
        ui.label(
            egui::RichText::new(&self.remote_path).small().color(egui::Color32::GRAY),
        );

        let mut navigate_idx = None;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                if !self.connected {
                    ui.label("Connecting...");
                    return;
                }
                for (i, entry) in self.remote_entries.iter().enumerate() {
                    let selected = self.selected_remote == Some(i);
                    let icon = if entry.is_dir { "[D]" } else { "   " };
                    let label = format!("{} {}  {}", icon, entry.name, format_size(entry.size));

                    let resp = ui.selectable_label(selected, label);
                    if resp.clicked() {
                        self.selected_remote = Some(i);
                    }
                    if resp.double_clicked() && entry.is_dir {
                        navigate_idx = Some(i);
                    }
                }
            });
        if let Some(idx) = navigate_idx {
            if let Some(entry) = self.remote_entries.get(idx) {
                let name = entry.name.clone();
                self.navigate_remote_into(&name);
            }
        }
    }

    fn refresh_local(&mut self) {
        let mut entries: Vec<LocalFileEntry> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.local_path) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    entries.push(LocalFileEntry {
                        name,
                        is_dir: meta.is_dir(),
                        size: meta.len(),
                    });
                }
            }
        }
        entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.local_entries = entries;
        self.selected_local = None;
    }

    fn navigate_local_into(&mut self, name: &str) {
        let mut new_path = self.local_path.clone();
        if !new_path.ends_with('\\') && !new_path.ends_with('/') {
            new_path.push('\\');
        }
        new_path.push_str(name);
        if std::path::Path::new(&new_path).is_dir() {
            self.local_path = new_path;
            self.refresh_local();
        }
    }

    fn navigate_local_up(&mut self) {
        if let Some(parent) = std::path::Path::new(&self.local_path).parent() {
            let p = parent.to_string_lossy().to_string();
            if !p.is_empty() {
                self.local_path = p;
                self.refresh_local();
            }
        }
    }

    fn navigate_remote_into(&mut self, name: &str) {
        let mut new_path = self.remote_path.clone();
        if !new_path.ends_with('/') {
            new_path.push('/');
        }
        new_path.push_str(name);
        self.remote_path = new_path;
        self.selected_remote = None;
        self.sftp.list_dir(&self.remote_path);
        self.pending_list = true;
    }

    fn navigate_remote_up(&mut self) {
        let path = std::path::Path::new(&self.remote_path);
        if let Some(parent) = path.parent() {
            let p = parent.to_string_lossy().to_string();
            if !p.is_empty() {
                self.remote_path = p;
                self.selected_remote = None;
                self.sftp.list_dir(&self.remote_path);
                self.pending_list = true;
            }
        }
    }

    fn do_upload(&mut self) {
        if let Some(idx) = self.selected_local {
            if let Some(entry) = self.local_entries.get(idx) {
                if entry.is_dir {
                    self.status = "Cannot upload directories".to_string();
                    return;
                }
                let local = format_local_path(&self.local_path, &entry.name);
                let remote = format_remote_path(&self.remote_path, &entry.name);
                self.status = format!("Uploading {}...", entry.name);
                self.sftp.upload(local, remote);
            }
        }
    }

    fn do_download(&mut self) {
        if let Some(idx) = self.selected_remote {
            if let Some(entry) = self.remote_entries.get(idx) {
                if entry.is_dir {
                    self.status = "Cannot download directories".to_string();
                    return;
                }
                let remote = format_remote_path(&self.remote_path, &entry.name);
                let local = format_local_path(&self.local_path, &entry.name);
                self.status = format!("Downloading {}...", entry.name);
                self.sftp.download(remote, local);
            }
        }
    }
}

fn format_size(size: u64) -> String {
    if size == 0 {
        return String::new();
    }
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

fn format_local_path(dir: &str, name: &str) -> String {
    let sep = if dir.contains('\\') { '\\' } else { '/' };
    format!("{}{}{}", dir.trim_end_matches(sep), sep, name)
}

fn format_remote_path(dir: &str, name: &str) -> String {
    format!("{}/{}", dir.trim_end_matches('/'), name)
}
