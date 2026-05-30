use crate::ui::split_pane::{SplitLayout, PaneKind};

pub struct Tab {
    pub id: String,
    pub title: String,
    pub layout: SplitLayout,
}

impl Tab {
    pub fn new_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let layout = SplitLayout::new_single_local(rows, cols, scrollback, shell)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Terminal".to_string(),
            layout,
        })
    }

    pub fn poll(&mut self) {
        self.layout.poll_all();
        if let Some(pane) = self.layout.active_pane() {
            if let PaneKind::Terminal { terminal, .. } = &pane.kind {
                if !terminal.title.is_empty() {
                    self.title = terminal.title.clone();
                }
            }
        }
    }

    pub fn alive(&self) -> bool {
        self.layout.panes.iter().any(|p| p.alive)
    }

    pub fn close(&mut self) {
        for pane in &mut self.layout.panes {
            pane.close();
        }
    }
}
