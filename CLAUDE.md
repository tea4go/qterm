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

## 窗口位置管理

支持记忆上次窗口位置，启动时自动恢复，支持命令行参数覆盖。

### 命令行参数

| 参数 | 示例 | 说明 |
|------|------|------|
| 无参数 | `qterm` | 恢复上次关闭时的位置和大小 |
| `--reset` | `qterm --reset` | 窗口重置为 1200×800，主屏居中 |
| `--setpos` | `qterm --setpos 3840,222` | 定位到指定物理像素坐标 |

优先级：`--reset` > `--setpos` > 配置文件恢复 > 系统默认

### 坐标单位

- `window_x` / `window_y`：**物理像素**（保存和读取时乘/除 `pixels_per_point()`）
- `window_width` / `window_height`：**egui points**（`inner_rect` / `with_inner_size` 直接使用）

### 延迟定位（第 2 帧）

窗口位置在 `update()` 的**第 2 帧**才通过 `ViewportCommand::OuterPosition` 设置，原因是第 1 帧 DPI 缩放因子尚未准确。

### 实现位置

- `src/main.rs` — 启动入口：命令行解析、`is_position_visible()`、`primary_monitor_center()`
- `src/app.rs` — 第 2 帧延迟定位（`frame_count == 2`）、每帧追踪位置、`on_exit()` 保存
- `src/config.rs` — `AppConfig` 读写 `window_x/y/width/height/maximized`

### 诊断日志

启动过程写入 `%APPDATA%/qterm/startup.log`，记录显示器信息、配置读取、位置可见性判断。

---

## egui 布局经验与陷阱

### SidePanel 宽度被内容撑大（黑色区域问题）

**现象**：左侧面板与终端区域之间出现大块黑色空白区域。

**根本原因**：egui `SidePanel` 虽然设置了 `.exact_width(N)`，但面板内部若有 widget 通过 `ui.label(galley)` 或 `ui.allocate_exact_size(vec2(ui.available_width(), h), ...)` 请求了超出 N 的布局宽度，SidePanel 的实际布局宽度会被撑大。SidePanel 的 frame 只绘制设定宽度范围，超出部分既不属于 SidePanel 也不属于 CentralPanel，窗口底层背景色（黑色）就会裸露出来。

**诊断方法**：
1. 在 `renderer::render` 里打印 `ui.available_rect_before_wrap().min.x`，若该值远大于 `LEFT_PANE_WIDTH`，说明 SidePanel 被撑大。
2. 把 SidePanel frame fill 改成红色，黑色区域不变红 → 确认是 SidePanel 布局超出 frame 区域；把 CentralPanel frame fill 改成红色，黑色区域不变红 → 确认不是 CentralPanel 背景。

**修复规则**：
- **禁止**在 SidePanel 内用 `ui.allocate_exact_size(vec2(ui.available_width(), h), ...)` —— 第一帧 `ui.available_width()` 可能返回整个窗口宽度。
  改为：`let w = ui.available_width().min(LEFT_PANE_WIDTH);`
- **禁止**在 SidePanel 内用 `ui.label(galley)` 渲染可能很长的文本（如路径、标签标题）—— label 会按 galley 实际宽度请求布局空间。
  改为：用 `ui.allocate_exact_size(vec2(固定宽, h), ...)` 分配区域 + `painter.text(...)` 直接绘制。
- **禁止** `ScrollArea::vertical().auto_shrink([false, true])`（水平不收缩），改为 `auto_shrink([true, true])` 并设置 `.max_width(LEFT_PANE_WIDTH)`。
- SidePanel 需加 `.resizable(false)`，防止 resize handle 占用额外布局空间。
- 在 SidePanel show closure 开头调用 `ui.set_clip_rect(ui.max_rect())` 做视觉兜底裁剪。