# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 提供本仓库代码工作时的指引。

## 构建与运行

```bash
cargo build                      # 调试构建
cargo build --release            # 发布构建（LTO + strip）
cargo check                      # 快速类型检查
cargo run                        # 构建 + 运行调试版
```

PowerShell 脚本（`build.ps1`）：
```powershell
.\build.ps1                      # 调试构建 + 运行
.\build.ps1 -Release             # 发布构建 + 运行
.\build.ps1 -BuildOnly           # 仅构建，不运行
.\build.ps1 -Clean               # 清理后重新构建 + 运行
```

暂无测试套件，使用 `cargo check` 进行快速验证。

## 架构

QTerm 是一个 GPU 加速的终端模拟器，使用 Rust + egui/eframe 构建。UI 使用自定义标题栏（无原生窗口装饰），左侧边栏包含图标栏 + 连接列表，中央终端区域支持分屏。

### 数据流

```
main.rs → QTermApp (app.rs)
  ├── Tab (tab/tab_item.rs) — 拥有 SplitLayout
  │     └── SplitLayout (ui/split_pane.rs) — 管理子面板（最多6个）
  │           └── ChildPane::PaneKind
  │                 ├── Terminal { terminal, backend: PtyHandle | SshHandle }
  │                 └── Sftp { panel }
  ├── AppConfig (config.rs) — 窗口状态，保存到 APPDATA/qterm/config.ini
  ├── Preferences (config.rs) — 字体/主题，来自 APPDATA/WhaleTerm/preferences.json
  └── AppTheme (theme/) — SystemTheme + TerminalTheme + ExtraTheme
```

### 终端数据管线

1. **PTY/SSH** → 通过通道（`reader_rx`）传输原始字节
2. **Terminal::feed()** → `vte::Parser` → 更新 `Grid`（含字符+颜色+属性的单元格）
3. **renderer::render()** → 读取 `Grid`，使用 `TerminalTheme` 颜色通过 egui `Painter` 绘制
4. **用户输入** → 键盘/鼠标事件 → 将字节写回 PTY/SSH

### 关键模块

- **`terminal/`** — Grid（回滚缓冲区），Cell（字符+ANSI属性），Parser（VTE转义序列），Renderer（egui绘制）
- **`theme/`** — `SystemTheme`（UI颜色，应用到 egui Style），`TerminalTheme`（ANSI 16/256色，光标），`ExtraTheme`（SFTP进度条，表格）。所有颜色为硬编码十六进制值，通过 `parse_color()` 解析。
- **`ssh/`** — `SshHandle` 封装 russh 配合 tokio 运行时，`SshClient` 为 russh Handler
- **`sftp/`** — `SftpHandle` 封装 russh-sftp，从现有 SSH 连接打开
- **`pty/`** — `PtyHandle` 封装 portable-pty 用于本地 Shell 会话
- **`connection/`** — 读取 WhaleTerm 的 `connections.json`，解密 AES-256-CFB 密码（密钥由主板序列号派生）

### 配置来源

| 文件 | 位置 | 用途 |
|------|------|------|
| `config.ini` | `APPDATA/qterm/` | 窗口位置/大小，字体缩放级别 |
| `preferences.json` | `APPDATA/WhaleTerm/` | 各区域字体族/大小/粗体，主题 |
| `connections.json` | `APPDATA/WhaleTerm/` | SSH 连接配置及加密密码 |

字体配置映射（来自 `preferences.json`）：
- `config.defaultFontFamily/Size/Bold` → 主体默认字体
- `general.fontFamily/Size/Bold` → 左侧边栏/大纲字体
- `shell.fontFamily/Size/Bold` → 终端和 SFTP 字体

### UI 布局（在 `app.rs` update 循环中渲染）

```
┌─ 标题栏 (40px，自定义拖拽区 + 窗口控制按钮) ─────────────┐
│ [QTerm] [标签1] [标签2] [+]                    [-][□][x]  │
├────┬─────────────┬───────────────────────────────────────┤
│ >_ │ 连接        │ 终端 / SFTP 面板                       │
│  F │  分组 1     │                                       │
│    │   host1     │  （最多6个分屏面板，水平或垂直）        │
│    │   host2     │                                       │
│    │ 打开的标签  │                                       │
│    │  tab1       │                                       │
│    │             │                                       │
│ [L]│             │                                       │
├────┴─────────────┴───────────────────────────────────────┤
│ ● 会话 | 已连接    Ctrl+T 新建 | Ctrl+Shift+N SSH ...    │
└──────────────────────────────────────────────────────────┘
```

### 快捷键

- `Ctrl+T` / `Ctrl+W` — 新建/关闭标签页
- `Ctrl+Shift+H/V` — 水平/垂直分屏
- `Ctrl+Shift+W` — 关闭活动面板
- `Ctrl+方向键` — 切换面板
- `Ctrl+B` — 切换左侧边栏
- `Ctrl+Shift+N` — SSH 连接对话框
- `Ctrl+Shift+F` — 从活动 SSH 面板打开 SFTP
- `Ctrl+/-` — 字体缩放
- 左侧图标栏 `L/D` 按钮 — 切换浅色/深色主题