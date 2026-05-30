# Phase 2 设计规格：SSH 连接 + 终端分屏

## 概述

在 Phase 1（本地终端）基础上，添加 SSH 远程连接和终端分屏功能。

## 范围

- SSH 远程连接（密码/密钥认证）
- 终端分屏（水平/垂直，最多 6 个 pane）
- Tab 结构改造（支持多 pane）
- SSH 连接对话框（基础 UI）

不包含：连接管理/侧边栏/SFTP/隧道/RDP。

## 技术选型

| 组件 | 选择 | 说明 |
|------|------|------|
| SSH | russh 0.46 | 纯 Rust，async 原生 |
| 异步 | tokio (rt-multi-thread) | 仅在 SSH 模块内部使用 |
| 密钥 | russh-keys 0.46 | RSA/ED25519 密钥解析 |

## 架构设计

### SSH 模块 (`src/ssh/`)

内部持有一个共享 tokio Runtime（OnceLock），所有 SSH 连接在此 runtime 上运行。
对外暴露 `SshHandle`，接口与 `PtyHandle` 一致：

```rust
pub struct SshHandle {
    pub reader_rx: Receiver<Vec<u8>>,
    writer_tx: Sender<Vec<u8>>,
    resize_tx: Sender<(u16, u16)>,
    alive: Arc<AtomicBool>,
}

impl SshHandle {
    pub fn connect(config: SshConfig) -> Result<Self, SshError>;
    pub fn write(&self, data: &[u8]) -> Result<()>;
    pub fn resize(&self, rows: u16, cols: u16);
    pub fn is_alive(&self) -> bool;
    pub fn disconnect(&self);
}
```

### SSH 连接配置

```rust
pub struct SshConfig {
    pub host: String,
    pub port: u16,            // 默认 22
    pub username: String,
    pub auth: SshAuth,
    pub timeout_secs: u32,    // 默认 5
}

pub enum SshAuth {
    Password(String),
    PrivateKey { path: String, passphrase: Option<String> },
}
```

### SSH 内部流程

1. `SshHandle::connect()` 在共享 tokio runtime 上 spawn 一个 async task
2. async task 内部：TCP 连接 → SSH 握手 → 认证 → 请求 PTY → 启动 shell
3. 启动两个子任务：reader（SSH channel → mpsc tx）和 writer（mpsc rx → SSH channel）
4. resize 通过独立 channel 发送 window-change 请求

### 分屏模块 (`src/ui/split_pane.rs`)

```rust
pub enum SplitDirection { Horizontal, Vertical }

pub struct SplitLayout {
    pub panes: Vec<ChildPane>,
    pub direction: SplitDirection,
    pub active_pane: usize,
}

pub struct ChildPane {
    pub id: String,
    pub terminal: Terminal,
    pub backend: PaneBackend,
}

pub enum PaneBackend {
    Local(PtyHandle),
    Ssh(SshHandle),
}
```

SplitLayout 方法：
- `add_pane(direction)` — 添加新 pane（最多 6 个）
- `remove_pane(idx)` — 关闭指定 pane
- `active_pane_mut()` — 获取当前活跃 pane
- `poll_all()` — 轮询所有 pane 的输出
- `resize_all(total_width, total_height)` — 按比例分配尺寸

### Tab 结构改造

```rust
pub struct Tab {
    pub id: String,
    pub title: String,
    pub layout: SplitLayout,  // 替代原来的 terminal + pty
}
```

### SSH 连接对话框

简单的 egui Window 弹窗：
- 输入框：Host、Port、Username、Password
- 单选：密码认证 / 密钥认证（密钥选择文件路径）
- 按钮：连接 / 取消
- 状态：连接中... / 错误信息

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+Shift+H | 水平分屏 |
| Ctrl+Shift+V | 垂直分屏 |
| Ctrl+方向键 | 切换活跃 pane |
| Ctrl+Shift+W | 关闭当前 pane |
| Ctrl+Shift+N | 新建 SSH 连接 |

## 文件结构变更

```
src/
├── ssh/
│   ├── mod.rs          # SshHandle + 共享 Runtime
│   ├── client.rs       # 连接建立 + 认证
│   └── session.rs      # PTY channel 读写循环
├── ui/
│   ├── mod.rs
│   ├── split_pane.rs   # SplitLayout + ChildPane
│   └── ssh_dialog.rs   # SSH 连接对话框
├── tab/
│   ├── mod.rs
│   └── tab_item.rs     # 改造为使用 SplitLayout
└── (其他文件不变)
```

## 新增依赖

```toml
russh = "0.46"
russh-keys = "0.46"
tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync"] }
```

## 验证标准

1. 能通过密码连接到 SSH 服务器并获得远程 shell
2. 远程终端输出正确渲染（颜色、光标）
3. 水平/垂直分屏正常工作，每个 pane 独立
4. 切换 pane 焦点后键盘输入发送到正确的 pane
5. 关闭 pane 时正确释放 SSH 连接
6. 连接超时/失败时显示错误信息
7. 内存占用：单个 SSH 连接 < 30MB 增量
