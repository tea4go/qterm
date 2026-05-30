# Phase 2: SSH 连接 + 终端分屏 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 为 qterm 添加 SSH 远程连接和终端分屏功能

**架构：** SSH 模块内部使用共享 tokio Runtime 运行 russh async 任务，对外通过 std::sync::mpsc channel 暴露与 PtyHandle 一致的接口。分屏通过 SplitLayout 管理多个 ChildPane，每个 pane 持有独立的 Terminal + Backend（Local 或 SSH）。

**技术栈：** egui 0.29, russh 0.46, russh-keys 0.46, tokio 1 (rt-multi-thread)

---

## 文件结构

| 文件 | 职责 |
|------|------|
| `Cargo.toml` | 添加 russh/tokio 依赖 |
| `src/main.rs` | 添加 `mod ssh; mod ui;` |
| `src/ssh/mod.rs` | SshHandle 公共接口 + 共享 Runtime + SshConfig/SshAuth 类型 |
| `src/ssh/client.rs` | SSH 连接建立 + 认证（async） |
| `src/ssh/session.rs` | PTY channel 读写循环（async） |
| `src/ui/mod.rs` | UI 子模块入口 |
| `src/ui/split_pane.rs` | SplitLayout + ChildPane + PaneBackend |
| `src/ui/ssh_dialog.rs` | SSH 连接对话框 UI |
| `src/tab/tab_item.rs` | 改造为使用 SplitLayout |
| `src/app.rs` | 集成分屏渲染 + SSH 对话框 + 新快捷键 |
| `tests/ssh_handle_test.rs` | SSH 模块集成测试 |
| `tests/split_pane_test.rs` | 分屏逻辑单元测试 |

---

## 任务 1：添加依赖 + 模块声明

**文件：**
- 修改：`Cargo.toml`
- 修改：`src/main.rs`
- 创建：`src/ssh/mod.rs`（空占位）
- 创建：`src/ui/mod.rs`（空占位）

- [ ] **步骤 1：修改 Cargo.toml 添加依赖**

```toml
[dependencies]
eframe = { version = "0.29", default-features = false, features = ["default_fonts", "wgpu"] }
egui = "0.29"
portable-pty = "0.9"
vte = "0.13"
uuid = { version = "1", features = ["v4"] }
russh = "0.46"
russh-keys = "0.46"
tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync"] }
```

- [ ] **步骤 2：创建空模块文件**

`src/ssh/mod.rs`:
```rust
pub mod client;
pub mod session;
```

`src/ui/mod.rs`:
```rust
pub mod split_pane;
pub mod ssh_dialog;
```

- [ ] **步骤 3：修改 src/main.rs 添加模块声明**

在现有 `mod` 声明后添加：
```rust
mod ssh;
mod ui;
```

- [ ] **步骤 4：创建空子模块文件使编译通过**

`src/ssh/client.rs`: 空文件
`src/ssh/session.rs`: 空文件
`src/ui/split_pane.rs`: 空文件
`src/ui/ssh_dialog.rs`: 空文件

- [ ] **步骤 5：运行 cargo check 确认编译通过**

运行：`cargo check`
预期：编译成功（可能有 unused 警告，无 error）

- [ ] **步骤 6：Commit**

```bash
git add Cargo.toml src/main.rs src/ssh/ src/ui/
git commit -m "feat: add ssh and ui module scaffolding with russh/tokio deps"
```

---

## 任务 2：实现 PaneBackend trait 抽象

**文件：**
- 创建：`src/ui/split_pane.rs`

为了让 SplitLayout 能统一管理本地终端和 SSH 终端，需要一个统一的 backend 接口。

- [ ] **步骤 1：定义 PaneBackend enum 和 ChildPane**

`src/ui/split_pane.rs`:
```rust
use std::sync::mpsc::Receiver;
use crate::pty::PtyHandle;
use crate::terminal::Terminal;

pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum PaneBackend {
    Local(PtyHandle),
}

pub struct ChildPane {
    pub id: String,
    pub terminal: Terminal,
    pub backend: PaneBackend,
    pub alive: bool,
}

pub struct SplitLayout {
    pub panes: Vec<ChildPane>,
    pub direction: SplitDirection,
    pub active_pane: usize,
}
```

- [ ] **步骤 2：实现 ChildPane 基础方法**

```rust
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
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        match &mut self.backend {
            PaneBackend::Local(pty) => { let _ = pty.write(data); }
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.terminal.resize(rows, cols);
        match &self.backend {
            PaneBackend::Local(pty) => pty.resize(rows as u16, cols as u16),
        }
    }

    pub fn close(&mut self) {
        match &mut self.backend {
            PaneBackend::Local(pty) => pty.kill(),
        }
        self.alive = false;
    }
}
```

- [ ] **步骤 3：实现 SplitLayout**

```rust
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
```

- [ ] **步骤 4：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 5：Commit**

```bash
git add src/ui/split_pane.rs
git commit -m "feat: implement SplitLayout and ChildPane with local backend"
```

---

## 任务 3：改造 Tab 使用 SplitLayout

**文件：**
- 修改：`src/tab/tab_item.rs`
- 修改：`src/app.rs`

- [ ] **步骤 1：重写 tab_item.rs**

```rust
use crate::ui::split_pane::SplitLayout;

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
            if !pane.terminal.title.is_empty() {
                self.title = pane.terminal.title.clone();
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
```

- [ ] **步骤 2：更新 app.rs 中对 Tab 的使用**

将 `app.rs` 中所有 `tab.terminal` 改为 `tab.layout.active_pane().unwrap().terminal`，
`tab.pty.write()` 改为 `tab.layout.active_pane_mut().unwrap().write()`，
`tab.pty.resize()` 改为 `tab.layout.active_pane_mut().unwrap().resize()`，
`tab.alive` 改为 `tab.alive()`。

关键修改点（`app.rs` 中的 `update` 方法）：

resize 部分：
```rust
if let Some(tab) = self.tabs.get_mut(self.active_tab) {
    if let Some(pane) = tab.layout.active_pane_mut() {
        pane.resize(size.rows, size.cols);
    }
}
```

render 部分：
```rust
if let Some(tab) = self.tabs.get(self.active_tab) {
    if let Some(pane) = tab.layout.active_pane() {
        renderer::render(ui, &pane.terminal, &self.theme);
    }
}
```

handle_input 部分：
```rust
fn handle_input(&mut self, ctx: &egui::Context) {
    let tab = match self.tabs.get_mut(self.active_tab) {
        Some(t) => t,
        None => return,
    };
    let pane = match tab.layout.active_pane_mut() {
        Some(p) => p,
        None => return,
    };
    // ... 将 tab.pty.write 改为 pane.write
}
```

tab bar 中 `tab.alive` 改为 `tab.alive()`。

- [ ] **步骤 3：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 4：运行程序验证本地终端仍正常工作**

运行：`cargo run`
预期：窗口打开，本地终端正常显示 prompt，可输入命令

- [ ] **步骤 5：Commit**

```bash
git add src/tab/tab_item.rs src/app.rs
git commit -m "refactor: tab uses SplitLayout instead of direct terminal+pty"
```

---

## 任务 4：实现分屏渲染

**文件：**
- 修改：`src/app.rs`

- [ ] **步骤 1：实现多 pane 渲染逻辑**

在 `app.rs` 的 CentralPanel 中，替换单 pane 渲染为分屏渲染：

```rust
// Central panel: terminal with split panes
egui::CentralPanel::default()
    .frame(egui::Frame::none().fill(self.theme.background))
    .show(ctx, |ui| {
        if self.tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("Press Ctrl+T to open a new terminal");
            });
            return;
        }

        let tab = &self.tabs[self.active_tab];
        let pane_count = tab.layout.pane_count();

        if pane_count == 1 {
            // 单 pane：全屏渲染
            let size = renderer::calculate_size(ui, self.theme.font_size);
            if let Some(pane) = tab.layout.active_pane() {
                renderer::render(ui, &pane.terminal, &self.theme);
            }
            // resize 逻辑...
        } else {
            // 多 pane：按方向分割
            match tab.layout.direction {
                SplitDirection::Horizontal => {
                    let available = ui.available_size();
                    let pane_height = available.y / pane_count as f32;
                    for (idx, pane) in tab.layout.panes.iter().enumerate() {
                        let is_active = idx == tab.layout.active_pane;
                        let frame = if is_active {
                            egui::Frame::none().fill(self.theme.background).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 200)))
                        } else {
                            egui::Frame::none().fill(self.theme.background)
                        };
                        frame.show(ui, |ui| {
                            ui.set_max_height(pane_height);
                            renderer::render(ui, &pane.terminal, &self.theme);
                        });
                    }
                }
                SplitDirection::Vertical => {
                    ui.horizontal(|ui| {
                        let available = ui.available_size();
                        let pane_width = available.x / pane_count as f32;
                        for (idx, pane) in tab.layout.panes.iter().enumerate() {
                            let is_active = idx == tab.layout.active_pane;
                            ui.vertical(|ui| {
                                ui.set_max_width(pane_width);
                                if is_active {
                                    ui.painter().rect_stroke(ui.max_rect(), 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 200)));
                                }
                                renderer::render(ui, &pane.terminal, &self.theme);
                            });
                        }
                    });
                }
            }
        }
    });
```

- [ ] **步骤 2：添加分屏快捷键处理**

在 `Action` enum 中添加：
```rust
enum Action {
    NewTab,
    CloseTab,
    NextTab,
    SplitHorizontal,
    SplitVertical,
    NextPane,
    ClosePane,
    OpenSshDialog,
}
```

在快捷键检测中添加：
```rust
if i.key_pressed(egui::Key::H) && i.modifiers.ctrl && i.modifiers.shift {
    action = Some(Action::SplitHorizontal);
}
if i.key_pressed(egui::Key::V) && i.modifiers.ctrl && i.modifiers.shift {
    action = Some(Action::SplitVertical);
}
if i.modifiers.ctrl && !i.modifiers.shift {
    if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowRight) {
        action = Some(Action::NextPane);
    }
}
if i.key_pressed(egui::Key::W) && i.modifiers.ctrl && i.modifiers.shift {
    action = Some(Action::ClosePane);
}
if i.key_pressed(egui::Key::N) && i.modifiers.ctrl && i.modifiers.shift {
    action = Some(Action::OpenSshDialog);
}
```

在 action 处理中添加：
```rust
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
        tab.layout.active_pane = (tab.layout.active_pane + 1) % tab.layout.pane_count();
    }
}
Some(Action::ClosePane) => {
    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
        let idx = tab.layout.active_pane;
        tab.layout.remove_pane(idx);
    }
}
```

- [ ] **步骤 3：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 4：运行程序测试分屏**

运行：`cargo run`
测试：按 Ctrl+Shift+H 水平分屏，确认出现两个独立终端

- [ ] **步骤 5：Commit**

```bash
git add src/app.rs
git commit -m "feat: implement split pane rendering and shortcuts"
```

---

## 任务 5：实现 SSH 模块核心

**文件：**
- 创建：`src/ssh/mod.rs`
- 创建：`src/ssh/client.rs`
- 创建：`src/ssh/session.rs`

- [ ] **步骤 1：实现 src/ssh/mod.rs — 类型定义 + 共享 Runtime**

```rust
pub mod client;
pub mod session;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};
use tokio::runtime::Runtime;

static SSH_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    SSH_RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create tokio runtime")
    })
}

#[derive(Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub timeout_secs: u32,
}

#[derive(Clone)]
pub enum SshAuth {
    Password(String),
    PrivateKey { path: String, passphrase: Option<String> },
}

#[derive(Debug)]
pub enum SshError {
    Connection(String),
    Auth(String),
    Channel(String),
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::Connection(e) => write!(f, "Connection error: {}", e),
            SshError::Auth(e) => write!(f, "Auth error: {}", e),
            SshError::Channel(e) => write!(f, "Channel error: {}", e),
        }
    }
}

impl std::error::Error for SshError {}

pub struct SshHandle {
    pub reader_rx: Receiver<Vec<u8>>,
    writer_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    resize_tx: tokio::sync::mpsc::Sender<(u16, u16)>,
    alive: Arc<AtomicBool>,
}

impl SshHandle {
    pub fn connect(config: SshConfig, rows: u16, cols: u16) -> Result<Self, SshError> {
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();
        let (writer_tx, writer_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
        let (resize_tx, resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(16);
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        let rt = get_runtime();
        let config_clone = config.clone();

        // Spawn the SSH session on the shared runtime
        // Use block_on in a std::thread to avoid blocking the GUI
        let alive_spawn = alive.clone();
        std::thread::spawn(move || {
            rt.block_on(async move {
                match session::run_ssh_session(config_clone, rows, cols, output_tx, writer_rx, resize_rx, alive_spawn).await {
                    Ok(()) => {}
                    Err(e) => eprintln!("SSH session error: {}", e),
                }
            });
            alive_clone.store(false, Ordering::Relaxed);
        });

        Ok(Self {
            reader_rx: output_rx,
            writer_tx,
            resize_tx,
            alive,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<(), SshError> {
        let _ = self.writer_tx.try_send(data.to_vec());
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.resize_tx.try_send((rows, cols));
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn disconnect(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}
```

- [ ] **步骤 2：实现 src/ssh/client.rs — Handler + 连接**

```rust
use russh::client::{Config, Handler, Handle};
use russh::keys::ssh_key;
use std::sync::Arc;

pub struct SshClient;

impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Phase 2: accept all keys (TODO: known_hosts in future)
        Ok(true)
    }
}

pub async fn connect_and_auth(
    config: &super::SshConfig,
) -> Result<Handle<SshClient>, super::SshError> {
    let ssh_config = Arc::new(Config::default());
    let addr = format!("{}:{}", config.host, config.port);

    let mut session = russh::client::connect(ssh_config, &*addr, SshClient)
        .await
        .map_err(|e| super::SshError::Connection(e.to_string()))?;

    let authenticated = match &config.auth {
        super::SshAuth::Password(password) => {
            session
                .authenticate_password(&config.username, password)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
        super::SshAuth::PrivateKey { path, passphrase } => {
            let key = russh_keys::load_secret_key(path, passphrase.as_deref())
                .map_err(|e| super::SshError::Auth(format!("Key load error: {}", e)))?;
            let key_pair = Arc::new(key);
            session
                .authenticate_publickey(&config.username, key_pair)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
    };

    if !authenticated.success() {
        return Err(super::SshError::Auth("Authentication failed".to_string()));
    }

    Ok(session)
}
```

- [ ] **步骤 3：实现 src/ssh/session.rs — PTY 会话读写循环**

```rust
use russh::ChannelMsg;
use std::sync::mpsc::Sender;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc::Receiver;

use super::client;
use super::{SshConfig, SshError};

pub async fn run_ssh_session(
    config: SshConfig,
    rows: u16,
    cols: u16,
    output_tx: Sender<Vec<u8>>,
    mut writer_rx: Receiver<Vec<u8>>,
    mut resize_rx: Receiver<(u16, u16)>,
    alive: Arc<AtomicBool>,
) -> Result<(), SshError> {
    let session = client::connect_and_auth(&config).await?;

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    channel
        .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    channel
        .request_shell(true)
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    // Main loop: read from SSH, write to SSH, handle resize
    loop {
        if !alive.load(Ordering::Relaxed) {
            break;
        }

        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        if output_tx.send(data.to_vec()).is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | None => {
                        break;
                    }
                    _ => {}
                }
            }
            Some(data) = writer_rx.recv() => {
                if channel.data(&data[..]).await.is_err() {
                    break;
                }
            }
            Some((r, c)) = resize_rx.recv() => {
                let _ = channel.window_change(c as u32, r as u32, 0, 0).await;
            }
        }
    }

    alive.store(false, Ordering::Relaxed);
    let _ = channel.eof().await;
    let _ = session.disconnect(russh::Disconnect::ByApplication, "", "").await;
    Ok(())
}
```

- [ ] **步骤 4：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 5：Commit**

```bash
git add src/ssh/
git commit -m "feat: implement SSH module with russh (connect, auth, pty session)"
```

---

## 任务 6：将 SSH 集成到 PaneBackend

**文件：**
- 修改：`src/ui/split_pane.rs`

- [ ] **步骤 1：在 PaneBackend 中添加 Ssh 变体**

```rust
use crate::ssh::{SshHandle, SshConfig};

pub enum PaneBackend {
    Local(PtyHandle),
    Ssh(SshHandle),
}
```

- [ ] **步骤 2：更新 ChildPane 方法支持 SSH**

添加 `new_ssh` 构造函数：
```rust
impl ChildPane {
    pub fn new_ssh(config: SshConfig, rows: usize, cols: usize, scrollback: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ssh = SshHandle::connect(config, rows as u16, cols as u16)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            terminal: Terminal::new(rows, cols, scrollback),
            backend: PaneBackend::Ssh(ssh),
            alive: true,
        })
    }
}
```

更新 `poll`、`write`、`resize`、`close` 方法添加 `Ssh` 分支：
```rust
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
```

- [ ] **步骤 3：在 SplitLayout 添加 add_ssh_pane 方法**

```rust
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
```

- [ ] **步骤 4：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 5：Commit**

```bash
git add src/ui/split_pane.rs
git commit -m "feat: integrate SSH backend into PaneBackend"
```

---

## 任务 7：实现 SSH 连接对话框

**文件：**
- 创建：`src/ui/ssh_dialog.rs`
- 修改：`src/app.rs`

- [ ] **步骤 1：实现 SSH 对话框 UI**

`src/ui/ssh_dialog.rs`:
```rust
use crate::ssh::{SshAuth, SshConfig};

#[derive(PartialEq)]
enum AuthMode { Password, PrivateKey }

pub struct SshDialog {
    pub open: bool,
    host: String,
    port: String,
    username: String,
    password: String,
    key_path: String,
    key_passphrase: String,
    auth_mode: AuthMode,
    pub status: Option<String>,
    pub result: Option<SshConfig>,
}

impl SshDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            password: String::new(),
            key_path: String::new(),
            key_passphrase: String::new(),
            auth_mode: AuthMode::Password,
            status: None,
            result: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        egui::Window::new("SSH Connection")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.host);
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port);
                });
                ui.horizontal(|ui| {
                    ui.label("User:");
                    ui.text_edit_singleline(&mut self.username);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.auth_mode, AuthMode::Password, "Password");
                    ui.radio_value(&mut self.auth_mode, AuthMode::PrivateKey, "Private Key");
                });
                match self.auth_mode {
                    AuthMode::Password => {
                        ui.horizontal(|ui| {
                            ui.label("Password:");
                            ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                        });
                    }
                    AuthMode::PrivateKey => {
                        ui.horizontal(|ui| {
                            ui.label("Key file:");
                            ui.text_edit_singleline(&mut self.key_path);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Passphrase:");
                            ui.add(egui::TextEdit::singleline(&mut self.key_passphrase).password(true));
                        });
                    }
                }
                if let Some(status) = &self.status {
                    ui.colored_label(egui::Color32::RED, status);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Connect").clicked() {
                        self.try_connect();
                    }
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        self.status = None;
                    }
                });
            });
    }

    fn try_connect(&mut self) {
        let port: u16 = self.port.parse().unwrap_or(22);
        if self.host.is_empty() || self.username.is_empty() {
            self.status = Some("Host and username are required".to_string());
            return;
        }
        let auth = match self.auth_mode {
            AuthMode::Password => SshAuth::Password(self.password.clone()),
            AuthMode::PrivateKey => SshAuth::PrivateKey {
                path: self.key_path.clone(),
                passphrase: if self.key_passphrase.is_empty() { None } else { Some(self.key_passphrase.clone()) },
            },
        };
        self.result = Some(SshConfig {
            host: self.host.clone(),
            port,
            username: self.username.clone(),
            auth,
            timeout_secs: 5,
        });
        self.open = false;
        self.status = None;
    }
}
```

- [ ] **步骤 2：在 app.rs 中集成对话框**

在 `QTermApp` 中添加字段：
```rust
ssh_dialog: crate::ui::ssh_dialog::SshDialog,
```

在 `new()` 中初始化：
```rust
ssh_dialog: crate::ui::ssh_dialog::SshDialog::new(),
```

在 `update()` 中渲染对话框并处理结果：
```rust
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

// Handle OpenSshDialog action
Some(Action::OpenSshDialog) => {
    self.ssh_dialog.open = true;
}
```

- [ ] **步骤 3：运行 cargo check**

运行：`cargo check`
预期：编译成功

- [ ] **步骤 4：Commit**

```bash
git add src/ui/ssh_dialog.rs src/app.rs
git commit -m "feat: implement SSH connection dialog UI"
```

---

## 任务 8：端到端验证

**文件：** 无新文件

- [ ] **步骤 1：编译 release 版本**

运行：`cargo build --release`
预期：编译成功

- [ ] **步骤 2：测试本地终端**

启动程序，确认：
- 本地终端正常显示 prompt
- 可输入命令并看到输出
- Ctrl+T 新建 tab 正常

- [ ] **步骤 3：测试分屏**

- Ctrl+Shift+H 水平分屏 → 出现两个独立终端
- Ctrl+Shift+V 垂直分屏 → 出现垂直排列的终端
- Ctrl+方向键切换焦点 → 活跃 pane 边框高亮变化
- Ctrl+Shift+W 关闭当前 pane

- [ ] **步骤 4：测试 SSH 连接**

- Ctrl+Shift+N 打开 SSH 对话框
- 输入有效的 SSH 服务器信息
- 点击 Connect → 新 pane 出现远程 shell
- 在远程 shell 中执行命令确认正常

- [ ] **步骤 5：测试错误处理**

- 输入无效 host → 显示连接错误
- 输入错误密码 → 显示认证错误

- [ ] **步骤 6：最终 Commit**

```bash
git add -A
git commit -m "feat: phase 2 complete - SSH connection and split panes"
```

