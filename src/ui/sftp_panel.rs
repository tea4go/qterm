use eframe::egui;

use crate::sftp::{self, SftpEvent, FileEntry};

/// 本地文件条目（与远程 FileEntry 对应的本地版本）
struct LocalFileEntry {
    name: String,      // 文件名
    is_dir: bool,      // 是否为目录
    size: u64,         // 文件大小
}

/// SFTP 面板 UI 组件
/// 显示本地和远程文件的双栏浏览器，支持上传/下载操作
pub struct SftpPanel {
    sftp: sftp::SftpHandle,           // SFTP 客户端句柄
    local_path: String,               // 当前本地路径
    remote_path: String,              // 当前远程路径
    local_entries: Vec<LocalFileEntry>, // 本地文件列表
    remote_entries: Vec<FileEntry>,    // 远程文件列表
    selected_local: Option<usize>,     // 本地选中项索引
    selected_remote: Option<usize>,    // 远程选中项索引
    status: String,                   // 状态信息
    connected: bool,                  // 是否已连接
    pending_list: bool,               // 是否等待目录列表结果
}

impl SftpPanel {
    /// 创建 SFTP 面板实例
    /// 初始化本地路径为用户主目录，远程路径为根目录
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
            status: "正在连接...".to_string(),
            connected: false,
            pending_list: false,
        };
        panel.refresh_local();
        panel
    }

    /// 轮询 SFTP 事件并更新面板状态
    pub fn poll(&mut self) {
        for event in self.sftp.poll() {
            match event {
                SftpEvent::Connected => {
                    self.connected = true;
                    self.status = "已连接".to_string();
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
                            self.status = "上传完成".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("上传失败: {}", e),
                    }
                }
                SftpEvent::DownloadDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "下载完成".to_string();
                            self.refresh_local();
                        }
                        Err(e) => self.status = format!("下载失败: {}", e),
                    }
                }
                SftpEvent::MkdirDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "目录已创建".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("创建目录失败: {}", e),
                    }
                }
                SftpEvent::DeleteDone(result) => {
                    match result {
                        Ok(()) => {
                            self.status = "已删除".to_string();
                            self.sftp.list_dir(&self.remote_path);
                        }
                        Err(e) => self.status = format!("删除失败: {}", e),
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

    /// 显示 SFTP 面板 UI
    /// 左右双栏布局：本地文件浏览器 + 远程文件浏览器
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let available = ui.available_size();
            let half_w = available.x / 2.0;
            let btn_h = 28.0;
            let list_h = (available.y - btn_h).max(100.0);

            // 左右双栏布局
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

            // 底部操作栏
            ui.separator();
            ui.horizontal(|ui| {
                let can_upload = self.selected_local.is_some();
                let can_download = self.selected_remote.is_some();

                if ui.add_enabled(can_upload, egui::Button::new("上传 ->")).clicked() {
                    self.do_upload();
                }
                if ui.add_enabled(can_download, egui::Button::new("<- 下载")).clicked() {
                    self.do_download();
                }
                ui.separator();
                ui.label(&self.status);
            });
        });
    }

    /// 检查 SFTP 连接是否存活
    pub fn is_alive(&self) -> bool {
        self.sftp.is_alive()
    }

    /// 关闭 SFTP 连接
    pub fn close(&mut self) {
        self.sftp.disconnect();
    }

    /// 显示本地文件浏览器面板
    fn show_local_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("本地：");
            // 上级目录按钮
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
                    // 双击目录项进入子目录
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

    /// 显示远程文件浏览器面板
    fn show_remote_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("远程：");
            // 上级目录按钮
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
                    ui.label("正在连接...");
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
                    // 双击目录项进入子目录
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

    /// 刷新本地文件列表
    fn refresh_local(&mut self) {
        let mut entries: Vec<LocalFileEntry> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.local_path) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // 过滤隐藏文件（以.开头）
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
        // 排序：目录优先，然后按名称排序
        entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.local_entries = entries;
        self.selected_local = None;
    }

    /// 进入本地子目录
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

    /// 返回本地上级目录
    fn navigate_local_up(&mut self) {
        if let Some(parent) = std::path::Path::new(&self.local_path).parent() {
            let p = parent.to_string_lossy().to_string();
            if !p.is_empty() {
                self.local_path = p;
                self.refresh_local();
            }
        }
    }

    /// 进入远程子目录
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

    /// 返回远程上级目录
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

    /// 执行上传操作（本地 → 远程）
    fn do_upload(&mut self) {
        if let Some(idx) = self.selected_local {
            if let Some(entry) = self.local_entries.get(idx) {
                if entry.is_dir {
                    self.status = "无法上传目录".to_string();
                    return;
                }
                let local = format_local_path(&self.local_path, &entry.name);
                let remote = format_remote_path(&self.remote_path, &entry.name);
                self.status = format!("正在上传 {}...", entry.name);
                self.sftp.upload(local, remote);
            }
        }
    }

    /// 执行下载操作（远程 → 本地）
    fn do_download(&mut self) {
        if let Some(idx) = self.selected_remote {
            if let Some(entry) = self.remote_entries.get(idx) {
                if entry.is_dir {
                    self.status = "无法下载目录".to_string();
                    return;
                }
                let remote = format_remote_path(&self.remote_path, &entry.name);
                let local = format_local_path(&self.local_path, &entry.name);
                self.status = format!("正在下载 {}...", entry.name);
                self.sftp.download(remote, local);
            }
        }
    }
}

/// 格式化文件大小为人类可读格式（B/K/M/G）
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

/// 格式化本地文件路径（使用平台对应的分隔符）
fn format_local_path(dir: &str, name: &str) -> String {
    let sep = if dir.contains('\\') { '\\' } else { '/' };
    format!("{}{}{}", dir.trim_end_matches(sep), sep, name)
}

/// 格式化远程文件路径（始终使用 / 作为分隔符）
fn format_remote_path(dir: &str, name: &str) -> String {
    format!("{}/{}", dir.trim_end_matches('/'), name)
}