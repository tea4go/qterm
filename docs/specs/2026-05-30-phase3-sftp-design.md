# Phase 3 设计规格：SFTP 文件传输

## 概述

在 Phase 2（SSH + 分屏）基础上，添加 SFTP 文件传输功能。复用已有 SSH 连接，在分屏中以双栏面板形式展示本地和远程文件。

## 范围

- SFTP 客户端（复用 SSH session，通过 subsystem 建立）
- 双栏文件浏览器 UI（本地 + 远程）
- 文件上传/下载（带进度）
- 新建目录、删除、重命名
- 右键菜单操作

不包含：搜索、编辑远程文件、拖拽上传、断线重连。

## 技术选型

| 组件 | 选择 | 说明 |
|------|------|------|
| SFTP 协议 | russh-sftp 0.7 | 基于 russh 的 SFTP 实现 |
| 传输进度 | tokio channel | 进度事件通过 channel 传递 |

## 架构设计

### SSH 模块改造 (`src/ssh/`)

`SshHandle` 需要暴露 SSH session handle 以支持 SFTP subsystem：

```rust
// ssh/mod.rs 新增
impl SshHandle {
    pub fn open_sftp(&self) -> Result<SftpHandle, SshError>;
}
```

内部实现：将 `Handle<SshClient>` 用 `Arc<Mutex>` 共享，SFTP 模块通过它打开新 channel。

### SFTP 模块 (`src/sftp/`)

```rust
pub struct SftpHandle {
    pub events_rx: Receiver<SftpEvent>,
    cmd_tx: Sender<SftpCommand>,
}

pub enum SftpEvent {
    DirListing { path: String, entries: Vec<FileEntry> },
    TransferProgress { id: String, transferred: u64, total: u64 },
    TransferDone { id: String, result: Result<(), String> },
    Error(String),
    Connected,
}

pub enum SftpCommand {
    ListDir { path: String },
    Upload { local_path: String, remote_path: String },
    Download { remote_path: String, local_path: String },
    Mkdir { path: String },
    Delete { path: String, is_dir: bool },
    Rename { from: String, to: String },
    Disconnect,
}

pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}
```

### Pane 类型改造 (`src/ui/split_pane.rs`)

`ChildPane` 需要支持两种 pane 类型：

```rust
pub enum PaneKind {
    Terminal { terminal: Terminal, backend: PaneBackend },
    Sftp { panel: crate::ui::sftp_panel::SftpPanel },
}

pub struct ChildPane {
    pub id: String,
    pub kind: PaneKind,
    pub alive: bool,
}
```

### SFTP 面板 UI (`src/ui/sftp_panel.rs`)

```rust
pub struct SftpPanel {
    sftp: Option<SftpHandle>,
    local_path: String,
    remote_path: String,
    local_entries: Vec<FileEntry>,
    remote_entries: Vec<FileEntry>,
    transfers: Vec<TransferItem>,
    selected_local: Option<usize>,
    selected_remote: Option<usize>,
    status: Option<String>,
}

pub struct TransferItem {
    pub id: String,
    pub filename: String,
    pub direction: TransferDirection,
    pub transferred: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub enum TransferDirection { Upload, Download }
```

### 界面布局

双栏水平排列，底部传输列表：

```
┌────────────────────┬────────────────────┐
│ 📁 本地文件        │ 📁 远程文件        │
│ 路径: /home/user   │ 路径: /var/www     │
│ ┌────────────────┐ │ ┌────────────────┐ │
│ │ [..]           │ │ │ [..]           │ │
│ │ documents/     │ │ │ html/          │ │
│ │ file.txt  1.2K │ │ │ config.yml 2K  │ │
│ └────────────────┘ │ └────────────────┘ │
│ [上传 →] [← 下载]                      │
├─────────────────────────────────────────┤
│ 传输列表:                               │
│ file.txt  ██████████░░ 80%  上传中      │
│ data.zip  ████████████ 100% 完成        │
└─────────────────────────────────────────┘
```

### 新增文件

```
src/
├── sftp/
│   ├── mod.rs          # SftpHandle + 类型定义
│   ├── client.rs       # SFTP subsystem 连接 + 操作
│   └── transfer.rs     # 文件传输（分块读写 + 进度）
└── ui/
    └── sftp_panel.rs   # 双栏面板 UI
```

### 新增依赖

```toml
russh-sftp = "0.7"
```

### 交互流程

1. 在 SSH 终端 pane 中右键 → 选择"SFTP" → 在当前 tab 中添加 SFTP pane
2. SFTP pane 自动复用当前 SSH 连接，打开远程 `/`
3. 本地路径默认为用户 home 目录
4. 双击目录进入，单击选中文件
5. 点击"上传"/"下载"按钮执行传输
6. 传输进度实时显示在底部列表

### 验证标准

1. 能通过已有 SSH 连接打开 SFTP subsystem
2. 远程文件列表正确显示（目录/文件/大小）
3. 上传文件到远程服务器成功
4. 从远程下载文件到本地成功
5. 传输进度实时更新
6. 创建目录、删除文件操作正常
7. 关闭 SFTP pane 不影响 SSH 终端连接
