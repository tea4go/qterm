# QTerm - 跨平台终端模拟器

QTerm 是一个基于 Rust + egui/eframe 构建的轻量级 GPU 加速终端模拟器，支持本地终端、SSH 远程连接和 SFTP 文件传输。

## 功能特性

- **本地终端**：支持 Windows (PowerShell/CMD)、macOS (Zsh)、Linux (Bash)
- **SSH 远程连接**：支持密码认证和私钥认证
- **SFTP 文件传输**：基于 SSH 连接的双栏文件浏览器（上传/下载）
- **分屏支持**：最多 6 个面板，支持水平和垂直分屏
- **多标签页**：多个终端标签页独立管理
- **主题切换**：深色（Solarized Dark）和浅色（Light Modern）两种主题
- **中文支持**：自动加载 CJK 字体（微软雅黑/PingFang/Noto Sans CJK）
- **WhaleTerm 兼容**：读取 WhaleTerm 的连接配置和字体偏好设置

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

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+T` | 新建标签页 |
| `Ctrl+W` | 关闭标签页 |
| `Ctrl+Tab` | 切换到下一个标签页 |
| `Ctrl+Shift+H` | 水平分屏 |
| `Ctrl+Shift+V` | 垂直分屏 |
| `Ctrl+Shift+W` | 关闭活动面板 |
| `Ctrl+方向键` | 切换活动面板 |
| `Ctrl+Shift+N` | 打开 SSH 连接对话框 |
| `Ctrl+Shift+F` | 从 SSH 标签页打开 SFTP |
| `Ctrl+B` | 切换左侧面板显示 |
| `Ctrl+ +/-` | 终端字体缩放 |
| `Ctrl+C` | 复制选中文本 / 发送 SIGINT |
| `Ctrl+Shift+C` | 强制复制选中文本 |
| `Ctrl+V` | 粘贴 |
| 右键菜单 | 复制/粘贴/清屏/分屏 |

## 配置文件

| 文件 | 位置 | 用途 |
|------|------|------|
| `config.ini` | `%APPDATA%\qterm\` | 窗口位置/大小、字体缩放级别 |
| `preferences.json` | `%APPDATA%\WhaleTerm\` | 字体族/大小/粗体、主题设置 |
| `connections.json` | `%APPDATA%\WhaleTerm\` | SSH 连接配置（含 AES-256-CFB 加密密码） |

## 项目结构

```
src/
├── main.rs          # 应用入口
├── app.rs           # 主应用逻辑（UI渲染、事件处理）
├── config.rs        # 配置管理（AppConfig + Preferences）
├── connection/      # WhaleTerm 连接配置读取与密码解密
├── pty/             # 本地伪终端（portable-pty 封装）
├── ssh/             # SSH 连接管理（russh + tokio）
├── sftp/            # SFTP 文件传输（russh-sftp 封装）
├── tab/             # 标签页管理
├── terminal/        # 终端仿真器核心
│   ├── cell.rs      # 单元格（字符+颜色+属性）
│   ├── grid.rs      # 字符网格（含回滚缓冲区）
│   ├── parser.rs    # VTE ANSI 序列解析器
│   └── renderer.rs  # egui 终端渲染器
├── theme/           # 主题系统
│   ├── system.rs    # UI 控件颜色 + egui 样式应用
│   ├── terminal.rs  # ANSI 16/256色映射 + 光标/选区
│   └── extra.rs     # SFTP 进度条/表格颜色
└── ui/              # UI 组件
    ├── sftp_panel.rs # SFTP 双栏文件浏览器
    ├── split_pane.rs # 分屏布局管理
    └── ssh_dialog.rs # SSH 连接对话框
```

## 技术栈

- **Rust** — 主语言
- **egui / eframe** — 即时模式 GUI 框架
- **portable-pty** — 本地伪终端
- **vte** — ANSI 转义序列解析
- **russh** — SSH 客户端
- **russh-sftp** — SFTP 文件传输
- **tokio** — 异步运行时（SSH/SFTP）
- **serde / serde_json** — 配置文件解析
- **aes / cfb-mode** — AES-256-CFB 密码解密

## 许证

MIT