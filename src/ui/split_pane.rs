use crate::pty::PtyHandle;
use crate::ssh::{SshHandle, SshConfig};
use crate::terminal::Terminal;

#[derive(Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum PaneBackend {
    Local(PtyHandle),
    Ssh(SshHandle),
}

pub struct ChildPane {
    pub id: String,
    pub terminal: Terminal,
    pub backend: PaneBackend,
    pub alive: bool,
}

impl ChildPane {
    pub fn new_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty = PtyHandle::spawn(rows as u16, cols as u16, shell)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            terminal: Terminal::new(rows, cols, scrollback),
            backend: PaneBackend::Local(pty),
            alive: true,
        })
    }

    pub fn new_ssh(config: SshConfig, rows: usize, cols: usize, scrollback: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ssh = SshHandle::connect(config, rows as u16, cols as u16)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            terminal: Terminal::new(rows, cols, scrollback),
            backend: PaneBackend::Ssh(ssh),
            alive: true,
        })
    }

    pub fn poll(&mut self) {
        match &mut self.backend {
            PaneBackend::Local(pty) => {
                while let Ok(data) = pty.reader_rx.try_recv() {
                    self.terminal.feed(&data);
                }
                for reply in self.terminal.pending_replies.drain(..) {
                    let _ = pty.write(&reply);
                }
                if !pty.is_alive() {
                    self.alive = false;
                }
            }
            PaneBackend::Ssh(ssh) => {
                while let Ok(data) = ssh.reader_rx.try_recv() {
                    self.terminal.feed(&data);
                }
                for reply in self.terminal.pending_replies.drain(..) {
                    let _ = ssh.write(&reply);
                }
                if !ssh.is_alive() {
                    self.alive = false;
                }
            }
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        match &mut self.backend {
            PaneBackend::Local(pty) => { let _ = pty.write(data); }
            PaneBackend::Ssh(ssh) => { let _ = ssh.write(data); }
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.terminal.resize(rows, cols);
        match &self.backend {
            PaneBackend::Local(pty) => pty.resize(rows as u16, cols as u16),
            PaneBackend::Ssh(ssh) => ssh.resize(rows as u16, cols as u16),
        }
    }

    pub fn close(&mut self) {
        match &mut self.backend {
            PaneBackend::Local(pty) => pty.kill(),
            PaneBackend::Ssh(ssh) => ssh.disconnect(),
        }
        self.alive = false;
    }
}

pub struct SplitLayout {
    pub panes: Vec<ChildPane>,
    pub direction: SplitDirection,
    pub active_pane: usize,
}

impl SplitLayout {
    pub fn new_single_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pane = ChildPane::new_local(rows, cols, scrollback, shell)?;
        Ok(Self {
            panes: vec![pane],
            direction: SplitDirection::Horizontal,
            active_pane: 0,
        })
    }

    pub fn active_pane(&self) -> Option<&ChildPane> {
        self.panes.get(self.active_pane)
    }

    pub fn active_pane_mut(&mut self) -> Option<&mut ChildPane> {
        self.panes.get_mut(self.active_pane)
    }

    pub fn poll_all(&mut self) {
        for pane in &mut self.panes {
            pane.poll();
        }
    }

    pub fn add_local_pane(&mut self, direction: SplitDirection, rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        if self.panes.len() >= 6 {
            return Err("Maximum 6 panes reached".into());
        }
        let pane = ChildPane::new_local(rows, cols, scrollback, shell)?;
        self.panes.push(pane);
        self.direction = direction;
        self.active_pane = self.panes.len() - 1;
        Ok(())
    }

    pub fn add_ssh_pane(&mut self, config: SshConfig, direction: SplitDirection, rows: usize, cols: usize, scrollback: usize) -> Result<(), Box<dyn std::error::Error>> {
        if self.panes.len() >= 6 {
            return Err("Maximum 6 panes reached".into());
        }
        let pane = ChildPane::new_ssh(config, rows, cols, scrollback)?;
        self.panes.push(pane);
        self.direction = direction;
        self.active_pane = self.panes.len() - 1;
        Ok(())
    }

    pub fn remove_pane(&mut self, idx: usize) {
        if idx < self.panes.len() && self.panes.len() > 1 {
            self.panes[idx].close();
            self.panes.remove(idx);
            if self.active_pane >= self.panes.len() {
                self.active_pane = self.panes.len() - 1;
            }
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }
}
