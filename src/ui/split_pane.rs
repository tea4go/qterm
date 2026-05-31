use crate::pty::PtyHandle;
use crate::ssh::{SshHandle, SshConfig};
use crate::sftp::SftpHandle;
use crate::terminal::Terminal;

/// 分屏方向
#[derive(Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,  // 水平分屏（上下排列）
    Vertical,    // 垂直分屏（左右排列）
}

/// 面板后端类型（本地 PTY 或远程 SSH）
pub enum PaneBackend {
    Local(PtyHandle),  // 本地终端（PTY）
    Ssh(SshHandle),    // 远程终端（SSH）
}

/// 面板内容类型
pub enum PaneKind {
    Terminal { terminal: Terminal, backend: PaneBackend },  // 终端面板
    Sftp { panel: crate::ui::sftp_panel::SftpPanel },      // SFTP 文件浏览器面板
}

/// 子面板结构体
/// 管理单个面板的内容、存活状态和生命周期
pub struct ChildPane {
    pub id: String,      // 面板唯一标识
    pub kind: PaneKind,  // 面板内容类型
    pub alive: bool,     // 面板是否存活
}

impl ChildPane {
    /// 创建本地终端面板
    pub fn new_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty = PtyHandle::spawn(rows as u16, cols as u16, shell)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: PaneKind::Terminal {
                terminal: Terminal::new(rows, cols, scrollback),
                backend: PaneBackend::Local(pty),
            },
            alive: true,
        })
    }

    /// 创建 SSH 远程终端面板
    pub fn new_ssh(config: SshConfig, rows: usize, cols: usize, scrollback: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ssh = SshHandle::connect(config, rows as u16, cols as u16)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: PaneKind::Terminal {
                terminal: Terminal::new(rows, cols, scrollback),
                backend: PaneBackend::Ssh(ssh),
            },
            alive: true,
        })
    }

    /// 创建 SFTP 文件浏览器面板
    pub fn new_sftp(sftp: SftpHandle) -> Self {
        let panel = crate::ui::sftp_panel::SftpPanel::new(sftp);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: PaneKind::Sftp { panel },
            alive: true,
        }
    }

    /// 轮询面板数据（读取终端输出或 SFTP 事件）
    pub fn poll(&mut self) {
        match &mut self.kind {
            PaneKind::Terminal { terminal, backend } => {
                match backend {
                    PaneBackend::Local(pty) => {
                        // 读取本地 PTY 输出数据
                        while let Ok(data) = pty.reader_rx.try_recv() {
                            terminal.feed(&data);
                        }
                        // 发送终端待回复的 ANSI 响应
                        for reply in terminal.pending_replies.drain(..) {
                            let _ = pty.write(&reply);
                        }
                        // 检查 PTY 子进程是否存活
                        if !pty.is_alive() {
                            self.alive = false;
                        }
                    }
                    PaneBackend::Ssh(ssh) => {
                        // 读取 SSH 远程终端输出数据
                        while let Ok(data) = ssh.reader_rx.try_recv() {
                            terminal.feed(&data);
                        }
                        // 发送终端待回复的 ANSI 响应
                        for reply in terminal.pending_replies.drain(..) {
                            let _ = ssh.write(&reply);
                        }
                        // 检查 SSH 连接是否存活
                        if !ssh.is_alive() {
                            self.alive = false;
                        }
                    }
                }
            }
            PaneKind::Sftp { panel } => {
                // 轮询 SFTP 事件
                panel.poll();
                if !panel.is_alive() {
                    self.alive = false;
                }
            }
        }
    }

    /// 向面板写入数据（仅终端面板支持）
    pub fn write(&mut self, data: &[u8]) {
        if let PaneKind::Terminal { backend, .. } = &mut self.kind {
            match backend {
                PaneBackend::Local(pty) => { let _ = pty.write(data); }
                PaneBackend::Ssh(ssh) => { let _ = ssh.write(data); }
            }
        }
    }

    /// 调整面板终端大小（仅终端面板支持）
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if let PaneKind::Terminal { terminal, backend } = &mut self.kind {
            terminal.resize(rows, cols);
            match backend {
                PaneBackend::Local(pty) => pty.resize(rows as u16, cols as u16),
                PaneBackend::Ssh(ssh) => ssh.resize(rows as u16, cols as u16),
            }
        }
    }

    /// 关闭面板（终止后端进程/连接）
    pub fn close(&mut self) {
        match &mut self.kind {
            PaneKind::Terminal { backend, .. } => {
                match backend {
                    PaneBackend::Local(pty) => pty.kill(),
                    PaneBackend::Ssh(ssh) => ssh.disconnect(),
                }
            }
            PaneKind::Sftp { panel } => panel.close(),
        }
        self.alive = false;
    }
}

/// 分屏布局管理器
/// 管理多个面板的分屏排列和活动面板选择
pub struct SplitLayout {
    pub panes: Vec<ChildPane>,       // 面板列表（最多6个）
    pub direction: SplitDirection,    // 分屏方向
    pub active_pane: usize,          // 当前活动面板索引
}

impl SplitLayout {
    /// 创建包含单个本地终端面板的布局
    pub fn new_single_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pane = ChildPane::new_local(rows, cols, scrollback, shell)?;
        Ok(Self {
            panes: vec![pane],
            direction: SplitDirection::Horizontal,
            active_pane: 0,
        })
    }

    /// 获取当前活动面板的引用
    pub fn active_pane(&self) -> Option<&ChildPane> {
        self.panes.get(self.active_pane)
    }

    /// 获取当前活动面板的可变引用
    pub fn active_pane_mut(&mut self) -> Option<&mut ChildPane> {
        self.panes.get_mut(self.active_pane)
    }

    /// 轮询所有面板的数据
    pub fn poll_all(&mut self) {
        for pane in &mut self.panes {
            pane.poll();
        }
    }

    /// 添加本地终端面板到布局（最多6个面板）
    pub fn add_local_pane(&mut self, direction: SplitDirection, rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        if self.panes.len() >= 6 {
            return Err("已达到最大 6 个面板限制".into());
        }
        let pane = ChildPane::new_local(rows, cols, scrollback, shell)?;
        self.panes.push(pane);
        self.direction = direction;
        self.active_pane = self.panes.len() - 1;
        Ok(())
    }

    /// 添加 SSH 远程终端面板到布局（最多6个面板）
    pub fn add_ssh_pane(&mut self, config: SshConfig, direction: SplitDirection, rows: usize, cols: usize, scrollback: usize) -> Result<(), Box<dyn std::error::Error>> {
        if self.panes.len() >= 6 {
            return Err("已达到最大 6 个面板限制".into());
        }
        let pane = ChildPane::new_ssh(config, rows, cols, scrollback)?;
        self.panes.push(pane);
        self.direction = direction;
        self.active_pane = self.panes.len() - 1;
        Ok(())
    }

    /// 添加 SFTP 面板到布局（最多6个面板）
    pub fn add_sftp_pane(&mut self, sftp: SftpHandle, direction: SplitDirection) -> Result<(), Box<dyn std::error::Error>> {
        if self.panes.len() >= 6 {
            return Err("已达到最大 6 个面板限制".into());
        }
        let pane = ChildPane::new_sftp(sftp);
        self.panes.push(pane);
        self.direction = direction;
        self.active_pane = self.panes.len() - 1;
        Ok(())
    }

    /// 移除指定面板（至少保留1个面板）
    pub fn remove_pane(&mut self, idx: usize) {
        if idx < self.panes.len() && self.panes.len() > 1 {
            self.panes[idx].close();
            self.panes.remove(idx);
            if self.active_pane >= self.panes.len() {
                self.active_pane = self.panes.len() - 1;
            }
        }
    }

    /// 获取面板总数
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }
}