use crate::pty::PtyHandle;
use crate::terminal::Terminal;

pub struct Tab {
    pub id: String,
    pub title: String,
    pub terminal: Terminal,
    pub pty: PtyHandle,
    pub alive: bool,
}

impl Tab {
    pub fn new(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty = PtyHandle::spawn(rows as u16, cols as u16, shell)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Terminal".to_string(),
            terminal: Terminal::new(rows, cols, scrollback),
            pty,
            alive: true,
        })
    }

    pub fn poll(&mut self) {
        while let Ok(data) = self.pty.reader_rx.try_recv() {
            self.terminal.feed(&data);
        }
        // Send any pending replies back to PTY
        for reply in self.terminal.pending_replies.drain(..) {
            let _ = self.pty.write(&reply);
        }
        if !self.terminal.title.is_empty() {
            self.title = self.terminal.title.clone();
        }
        if !self.pty.is_alive() {
            self.alive = false;
        }
    }

    pub fn close(&mut self) {
        self.pty.kill();
        self.alive = false;
    }
}
