use crate::ui::split_pane::{SplitLayout, PaneKind};

/// 标签页结构体
/// 每个标签页包含一个分屏布局，可容纳多个终端或 SFTP 面板
pub struct Tab {
    pub id: String,          // 标签页唯一标识
    pub title: String,       // 标签页标题（通常由终端 OSC 标题设置）
    pub layout: SplitLayout, // 分屏布局管理器
}

impl Tab {
    /// 创建本地终端标签页
    /// 初始化一个包含单个本地终端面板的布局
    pub fn new_local(rows: usize, cols: usize, scrollback: usize, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let layout = SplitLayout::new_single_local(rows, cols, scrollback, shell)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "终端".to_string(),
            layout,
        })
    }

    /// 轮询标签页数据
    /// 读取终端输出，更新标签页标题
    pub fn poll(&mut self) {
        self.layout.poll_all();
        // 从终端 OSC 标题设置更新标签页标题
        if let Some(pane) = self.layout.active_pane() {
            if let PaneKind::Terminal { terminal, .. } = &pane.kind {
                if !terminal.title.is_empty() {
                    self.title = terminal.title.clone();
                }
            }
        }
    }

    /// 检查标签页是否存活（至少有一个面板存活）
    pub fn alive(&self) -> bool {
        self.layout.panes.iter().any(|p| p.alive)
    }

    /// 关闭标签页（关闭所有面板）
    pub fn close(&mut self) {
        for pane in &mut self.layout.panes {
            pane.close();
        }
    }
}