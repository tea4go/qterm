# WhaleTerminal 终端功能需求文档

## 1. 产品概述

WhaleTerminal 是一款功能丰富的终端模拟器，支持本地 Shell、SSH 远程连接、SFTP 文件传输、端口转发、RDP 远程桌面等功能。核心特性包括：多 Tab 管理、分屏终端、连接管理（分组/收藏/导入导出）、终端录制、系统监控、快捷命令等。

目标：用 Rust 重新实现一个原生终端桌面应用，**不使用 Webview2**，功能与 WhaleTerminal 保持一致。

---

## 2. 整体架构

### 2.1 原项目技术参考

| 层级 | 原项目技术 | 说明 |
|------|-----------|------|
| 前端 | Vue 3 + xterm.js 5.3.0 | 终端渲染 |
| 后端 | Go (Wails 框架) | PTY 管理、SSH、SFTP |
| PTY | go-console (Windows) / os/exec (Unix) | 本地终端 |
| SSH | golang.org/x/crypto/ssh | SSH 协议 |
| SFTP | github.com/pkg/sftp | SFTP 协议 |
| 加密 | AES-256-CFB | 密码存储加密 |

### 2.2 Rust 原生实现约束

- **不使用 Webview2**：终端渲染必须原生实现（GPU 加速文本渲染）
- 需要自行实现终端模拟器（VT100/VT220/xterm-256color 兼容）
- 需要原生 UI 框架（非 Web 技术）

### 2.3 应用布局

```
┌─────────────────────────────────────────────────────────────┐
│  标题栏 (Tab 栏 + 窗口控制)                                   │
├────────┬────────────────────────────────────────────────────┤
│ 侧边栏  │  Tab 内容区                                        │
│        │ ┌────────────────────────────────────────────────┐ │
│ 连接列表 │ │  终端工具栏 (可选: 快捷命令栏/命令输入框/监控)    │ │
│        │ ├────────────────────────────────────────────────┤ │
│ - 分组1 │ │                                                │ │
│   - 主机A│ │           终端输出区 (PTY 渲染)                 │ │
│   - 主机B│ │                                                │ │
│ - 分组2 │ │                                                │ │
│   - 主机C│ │                                                │ │
│        │ │                                                │ │
│ 收藏   │ │                                                │ │
│ 本地终端│ │                                                │ │
│        │ └────────────────────────────────────────────────┘ │
├────────┴────────────────────────────────────────────────────┤
│  状态栏                                                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. 终端模拟器核心

### 3.1 终端仿真要求

| 特性 | 要求 |
|------|------|
| 协议兼容 | VT100 / VT220 / xterm-256color |
| 颜色支持 | 16 色 + 256 色 + TrueColor (24-bit) |
| Unicode | Unicode 11+ 完整支持（含 CJK、Emoji） |
| 滚动缓冲 | 可配置，默认 3000 行（本地终端 1000 行） |
| 渲染方式 | GPU 加速文本渲染（非 Canvas/WebGL） |
| 光标样式 | Block / Underline / Bar，支持闪烁 |
| 选择模式 | 鼠标选择文本、双击选词、三击选行 |
| 换行处理 | 自动适配平台（CRLF/LF） |

### 3.2 终端转义序列支持

必须支持的转义序列类别：

| 类别 | 说明 |
|------|------|
| CSI 序列 | 光标移动、擦除、滚动、SGR 属性 |
| OSC 序列 | 窗口标题设置、剪贴板操作 |
| DCS 序列 | 设备控制（如 sixel 图形） |
| 鼠标事件 | X10/Normal/SGR 鼠标报告模式 |
| 括号粘贴 | Bracketed Paste Mode（\e[?2004h/l） |
| 替代屏幕 | Alt Screen Buffer（vim/less 等使用） |

### 3.3 文本渲染

| 特性 | 说明 |
|------|------|
| 字体 | 等宽字体，用户可配置字体族 |
| 字号 | 12-22px 可配置 |
| 加粗 | 支持粗体渲染 |
| 斜体 | 支持斜体渲染 |
| 下划线 | 支持下划线 |
| 删除线 | 支持删除线 |
| 反色 | 支持前景/背景色反转 |
| 闪烁 | 支持文本闪烁属性 |
| 连字 | 可选支持编程连字（Ligatures） |

### 3.4 输出缓冲与背压控制

- **写入队列**: FIFO 队列管理终端输出
- **最大缓冲**: 4MB 待处理数据上限
- **溢出处理**: 超出上限时截断旧数据，显示黄色警告提示
- **顺序写入**: 确保数据按序写入终端（回调式刷新）
- **自动清理**: 终端关闭时清空队列释放资源

---

## 4. 多 Tab 管理

### 4.1 Tab 架构

```
TabBar
├── Tab1 (SSH 连接)
│   ├── ChildTab1 (主终端)
│   └── ChildTab2 (分屏终端)
├── Tab2 (本地终端)
│   └── ChildTab1
├── Tab3 (SFTP)
└── Tab4 (RDP)
```

### 4.2 Tab 数据结构

```rust
struct TabItem {
    name: String,           // 唯一标识（UUID）
    title: String,          // 显示标题
    tab_type: TabType,      // SERVER / LOCAL / SFTP / RDP
    conn_id: String,        // 关联的连接 ID
    children: Vec<ChildTab>,// 子终端列表（分屏）
    active_child: String,   // 当前活跃的子终端
    term_layout: Layout,    // horizontal / vertical
    color: Option<String>,  // Tab 颜色标记
}

enum TabType {
    Server,     // SSH 远程连接
    Local,      // 本地终端
    Sftp,       // SFTP 文件管理
    Rdp,        // 远程桌面
}
```

### 4.3 Tab 操作

| 操作 | 说明 |
|------|------|
| 新建 Tab | 打开新连接或本地终端 |
| 关闭 Tab | 关闭当前 Tab（确认是否断开） |
| 关闭左侧 | 关闭当前 Tab 左侧所有 Tab |
| 关闭右侧 | 关闭当前 Tab 右侧所有 Tab |
| 重命名 | 修改 Tab 显示标题 |
| 复制连接 | 以相同配置打开新 Tab |
| 拖拽排序 | 拖拽调整 Tab 顺序 |
| 双击复制 | 双击 Tab 复制连接 |
| 颜色标记 | 为 Tab 设置颜色标识 |
| 溢出菜单 | Tab 过多时显示下拉列表 |

### 4.4 分屏终端

| 功能 | 说明 |
|------|------|
| 水平分屏 | 左右两列并排显示 |
| 垂直分屏 | 上下两行堆叠显示 |
| 最大分屏数 | 单个 Tab 最多 6 个子终端 |
| 切换焦点 | 方向键切换活跃分屏 |
| 关闭分屏 | 关闭单个分屏终端 |
| 独立会话 | 每个分屏有独立的终端会话 |
| 同步调整 | 窗口大小变化时所有分屏同步 resize |

---

## 5. 本地终端

### 5.1 支持的 Shell

| 平台 | 默认 Shell | 可选 Shell |
|------|-----------|-----------|
| Windows | PowerShell | cmd.exe, Git Bash, WSL |
| macOS | zsh | bash, fish |
| Linux | bash | zsh, fish, sh |

### 5.2 本地终端配置

```rust
struct LocalTerminal {
    name: String,           // 显示名称
    shell_path: String,     // Shell 可执行文件路径
    default_folder: String, // 启动目录
    start_cmd: String,      // 启动后执行的命令
    quick_cmd_line: String, // 快捷命令
    editable: bool,         // 是否可编辑配置
    hidden: bool,           // 是否在列表中隐藏
}
```

### 5.3 PTY 管理

| 特性 | 说明 |
|------|------|
| 创建 | 为每个本地终端创建独立 PTY |
| 环境变量 | 继承系统环境 + 设置 UTF-8 编码 |
| Resize | 终端大小变化时同步 PTY 窗口大小 |
| 关闭 | 关闭 Tab 时优雅终止 PTY 进程 |
| 编码 | 强制 UTF-8（Windows 设置 LANG=zh_CN.UTF-8） |

---

## 6. SSH 远程连接

### 6.1 连接建立流程

```
用户选择连接 → 读取连接配置 → 解密密码
      ↓
检查代理配置 → 建立 TCP 连接（直连/代理）
      ↓
SSH 握手 → 认证（密码/密钥）
      ↓
请求 PTY → 启动 Shell
      ↓
开始数据循环读取 → 事件推送到前端
```

### 6.2 认证方式

| 方式 | 说明 |
|------|------|
| 密码认证 | 加密存储，连接时解密使用 |
| 密钥认证 | 支持 RSA / ED25519 私钥 |
| 密钥生成 | 内置 ED25519 密钥对生成 |
| 免密登录 | 将公钥部署到远程 authorized_keys |

### 6.3 SSH 会话管理

| 特性 | 说明 |
|------|------|
| 连接池 | 同一主机复用 SSH 连接 |
| 多会话 | 单个 SSH 连接支持多个 Session |
| 心跳保活 | keepalive@openssh.com 定期发送 |
| 空闲超时 | 可配置自动关闭空闲连接 |
| 连接取消 | 支持取消正在建立的连接 |
| 超时设置 | 默认 5 秒，可配置 |
| 断线重连 | 断开后按 Enter 触发重连 |

### 6.4 跳板机（Bastion Host）

- 支持通过跳板机连接目标主机
- 跳板机配置独立于目标主机
- 自动检测跳板机跳转（目标主机识别）
- 每个连接使用唯一 UUID 标识

### 6.5 连接后操作

| 操作 | 说明 |
|------|------|
| 默认命令 | 连接成功后自动执行的命令列表 |
| 默认路径 | 连接后自动 cd 到指定目录 |
| 自动监控 | 可配置连接后自动开启系统监控 |

---

## 7. 代理与隧道

### 7.1 代理类型

| 类型 | 说明 |
|------|------|
| 无代理 | 直接连接 |
| 系统代理 | 使用系统环境变量中的代理 |
| 自定义代理 | 用户指定代理服务器 |
| 全局代理 | 应用级别全局代理 |

### 7.2 代理协议

| 协议 | 说明 |
|------|------|
| HTTP/HTTPS | HTTP CONNECT 隧道 |
| SOCKS5 | SOCKS5 代理协议 |

代理支持用户名/密码认证。

### 7.3 SSH 隧道（端口转发）

#### 本地端口转发 (Local Forward)
```
本地端口 → SSH 服务器 → 远程目标
LocalIP:LocalPort → SSHServer → RemoteIP:RemotePort
```
用途：通过 SSH 访问远程内网服务。

#### 远程端口转发 (Remote Forward)
```
远程端口 → SSH 服务器 → 本地目标
RemoteIP:RemotePort → SSHServer → LocalIP:LocalPort
```
用途：将本地服务暴露到远程网络。

#### 动态端口转发 (Dynamic Forward / SOCKS)
```
本地 SOCKS5 代理 → SSH 服务器 → 任意目标
```
用途：通过 SSH 建立 SOCKS5 代理，所有流量通过 SSH 转发。
支持 SOCKS5 认证（用户名/密码）。

### 7.4 隧道数据结构

```rust
struct ConnTunnel {
    local_tunnels: Vec<TunnelInfo>,     // 本地转发列表
    remote_tunnels: Vec<TunnelInfo>,    // 远程转发列表
    dynamic_tunnel: DynamicTunnelInfo,  // 动态转发
}

struct TunnelInfo {
    local_ip: String,
    local_port: u16,
    remote_ip: String,
    remote_port: u16,
}

struct DynamicTunnelInfo {
    local_port: u16,
    username: String,   // SOCKS 认证用户名
    password: String,   // SOCKS 认证密码
}
```

---

## 8. SFTP 文件传输

### 8.1 功能概述

SFTP 基于 SSH 连接提供安全文件传输，支持双面板（本地 + 远程）文件管理。

### 8.2 文件操作

| 操作 | 说明 |
|------|------|
| 浏览目录 | 远程/本地目录树浏览 |
| 上传文件 | 本地 → 远程，支持多文件 |
| 下载文件 | 远程 → 本地，支持多文件 |
| 创建目录 | 在远程创建新目录 |
| 删除 | 递归删除文件/目录（带进度） |
| 重命名 | 文件/目录重命名 |
| 复制/移动 | 文件复制和移动 |
| 权限修改 | chmod 操作 |
| 搜索 | 深度目录遍历搜索（支持正则、大小写） |
| 编辑文件 | 在线编辑远程文件（限制 5MB） |

### 8.3 传输特性

| 特性 | 说明 |
|------|------|
| 进度显示 | 实时传输进度百分比 |
| 任务取消 | 支持取消进行中的传输 |
| 冲突处理 | 重命名 / 覆盖 / 重试 三种策略 |
| 断线重连 | SFTP 超时后自动重连 |
| 跳板机支持 | 通过跳板机进行 SFTP |

### 8.4 界面布局

```
┌─────────────────────┬─────────────────────┐
│    本地文件面板      │    远程文件面板      │
│                     │                     │
│ 路径: /home/user    │ 路径: /var/www      │
│ ┌─────────────────┐ │ ┌─────────────────┐ │
│ │ 📁 documents    │ │ │ 📁 html         │ │
│ │ 📁 downloads    │ │ │ 📁 logs         │ │
│ │ 📄 config.yml   │ │ │ 📄 index.html   │ │
│ └─────────────────┘ │ └─────────────────┘ │
│                     │                     │
│ [上传 →]           │ [← 下载]           │
└─────────────────────┴─────────────────────┘
│              传输任务列表                   │
└────────────────────────────────────────────┘
```

---

## 9. 连接管理

### 9.1 连接数据结构

```rust
struct Connection {
    conn_id: String,            // UUID
    conn_type: ConnType,        // Normal / Cloud / DevCloud
    name: String,               // 连接名称
    addr: String,               // 主机地址
    port: u16,                  // 端口号（默认 22）
    username: String,           // 用户名
    password: String,           // 加密后的密码
    password_hint: String,      // 密码提示
    auth_model: AuthModel,      // Password / PrivateKey
    pub_key: String,            // 公钥名称
    pri_key: String,            // 私钥内容/路径
    description: String,        // 连接描述
    
    // 终端配置
    start_cmd: String,          // 连接后执行的启动命令
    default_cmds: Vec<String>,  // 默认命令列表
    quick_cmd_line: String,     // 快捷命令行
    
    // 文件管理
    local_path: String,         // 本地默认路径
    remote_path: String,        // 远程默认路径
    remote_favs: Vec<String>,   // 远程收藏路径
    show_hide_file: bool,       // 显示隐藏文件
    
    // 外观
    tab_type: TabType,          // Tab 类型（影响颜色）
    title_type: TitleType,      // 标题显示方式
    
    // 高级
    proxy: ConnProxy,           // 代理配置
    tunnel: ConnTunnel,         // 隧道配置
    code_server: CodeServer,    // Code-Server 配置
    conn_timeout: u32,          // 连接超时（秒）
    sysinfo: ConnSysinfo,      // 系统监控配置
    
    last_modify_time: i64,      // 最后修改时间
}

enum AuthModel {
    Password,
    PrivateKey,
}

enum ConnType {
    Normal = 0,
    Cloud = 1,
    DevCloud = 2,
}
```

### 9.2 连接分组

```rust
struct ConnectionGroup {
    group_id: String,
    group_name: String,
    connections: Vec<Connection>,
    sub_groups: Vec<ConnectionGroup>,  // 支持嵌套分组
}
```

### 9.3 连接操作

| 操作 | 说明 |
|------|------|
| 新建连接 | 填写连接信息创建 |
| 编辑连接 | 修改已有连接配置 |
| 删除连接 | 删除连接（确认） |
| 复制连接 | 复制连接配置 |
| 测试连接 | 验证连接可达性 |
| 收藏 | 添加到收藏列表 |
| 分组管理 | 创建/重命名/删除分组 |
| 拖拽排序 | 拖拽调整连接和分组顺序 |

### 9.4 导入导出

| 格式 | 说明 |
|------|------|
| JSON 导出 | 导出所有连接为 JSON 文件 |
| JSON 导入 | 从 JSON 文件导入连接 |
| XShell 导入 | 导入 XShell .xsh 配置文件 |

#### XShell 导入细节
- 解析 .xsh 文件格式（INI 风格）
- 密码解密：RC4 + SHA256（密钥 = 反转 UID + 用户名）
- 自动映射字段到内部连接结构

### 9.5 密码加密

| 项目 | 说明 |
|------|------|
| 算法 | AES-256-CFB |
| IV | 随机生成 |
| 密钥来源 | 主板序列号（硬件绑定） |
| 备用密钥 | 固定字符串（硬件信息不可用时） |
| 存储 | 密文 + IV 一起存储 |

---

## 10. 快捷命令系统

### 10.1 收藏命令

```rust
struct CmdFavorite {
    cmd_id: String,         // UUID
    cmd_name: String,       // 命令名称
    cmd_text: String,       // 命令内容
    os_type: String,        // 适用操作系统
    quick: bool,            // 是否显示在快捷栏
}

struct CmdFavsGroup {
    group_name: String,
    commands: Vec<CmdFavorite>,
}
```

### 10.2 快捷命令栏 (QuickCmdBar)

- 显示在终端上方的可拖拽按钮栏
- 点击按钮直接发送命令到终端
- 支持拖拽重新排序
- 可显示/隐藏
- 空状态提示添加命令

### 10.3 快捷命令输入框 (QuickCmdLine)

| 功能 | 说明 |
|------|------|
| 多行输入 | 支持多行命令编辑 |
| 反斜杠转换 | 可切换 `\` 到换行的转换 |
| 发送模式 | 逐行发送 / 合并发送 |
| 发送快捷键 | Enter 发送 / Ctrl+Enter 发送 |
| 可展开/折叠 | 高度可拖拽调整 |
| 停止按钮 | 中断正在执行的命令 |

---

## 11. 系统监控

### 11.1 监控面板

连接远程主机后可开启系统监控面板，实时显示：

| 指标 | 说明 |
|------|------|
| CPU | CPU 使用率 |
| 内存 | 内存使用量/总量 |
| 磁盘 | 磁盘 I/O 速率 |
| 网络 | 上传/下载带宽 |
| 进程 | Top N 进程列表 |

### 11.2 监控配置

| 配置 | 说明 |
|------|------|
| 自动启动 | 连接后自动开启监控 |
| 刷新间隔 | 可配置刷新频率 |
| 面板大小 | 可选面板尺寸 |
| 可折叠 | 抽屉式展开/收起 |

### 11.3 实现方式

- 通过 SSH 在远程执行 `sysinfo` 工具采集数据
- 支持安装监控工具到远程主机（Install Tools 功能）
- 数据格式化后推送到前端显示

---

## 12. 右键上下文菜单

### 12.1 终端区域右键菜单

| 菜单项 | 说明 |
|------|------|
| 复制 | 复制选中文本 |
| 粘贴 | 粘贴剪贴板内容 |
| 粘贴选中 | 粘贴当前选中文本 |
| 清屏 | 清除终端输出 |
| 全选 | 选中所有终端内容 |
| 重新连接 | 断开后重新连接 |
| 设置终端宽度 | 调整终端列数 |
| 分屏-水平 | 水平分割终端 |
| 分屏-垂直 | 垂直分割终端 |
| 收藏命令 | 显示收藏命令面板 |
| 快捷命令栏 | 显示/隐藏快捷命令栏 |
| 快捷命令行 | 显示/隐藏命令输入框 |
| 主机信息 | 显示/隐藏监控面板 |
| 安装工具 | 安装远程监控工具 |
| Code-Server | 启动远程 IDE |
| SFTP | 打开 SFTP 文件管理 |
| 免密登录 | 配置 SSH 密钥登录 |

### 12.2 快捷编辑模式

当右键时有选中文本：
- 自动复制选中文本
- 如果剪贴板有内容则自动粘贴

可在设置中切换为标准右键菜单模式。

### 12.3 Tab 右键菜单

| 菜单项 | 说明 |
|------|------|
| 新建连接 | 打开新连接 |
| 复制连接 | 复制当前连接 |
| 重命名 | 修改 Tab 标题 |
| 颜色标记 | 设置 Tab 颜色 |
| 关闭 | 关闭当前 Tab |
| 关闭左侧 | 关闭左侧所有 |
| 关闭右侧 | 关闭右侧所有 |

---

## 13. 键盘快捷键

### 13.1 终端快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+C | 有选中文本时复制，否则发送 SIGINT |
| Ctrl+V | 粘贴剪贴板内容到终端 |
| Ctrl+Shift+C | 强制复制 |
| Ctrl+Shift+V | 强制粘贴 |
| F6-F12 | 可配置的自定义快捷键 |

### 13.2 自定义快捷键

- F6-F12 可绑定为自定义命令
- 按下时发送对应的 ESC 序列或自定义文本
- 在设置中配置绑定内容

### 13.3 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+Tab | 切换到下一个 Tab |
| Ctrl+Shift+Tab | 切换到上一个 Tab |
| Ctrl+W | 关闭当前 Tab |
| Ctrl+T | 新建 Tab |

---

## 14. 主题与外观

### 14.1 应用主题

| 模式 | 说明 |
|------|------|
| 亮色主题 | 浅色背景 |
| 暗色主题 | 深色背景 |
| 跟随系统 | 自动跟随系统暗色模式 |

### 14.2 终端配色方案

```rust
struct XtermTheme {
    // 基础色
    foreground: String,         // 前景色（文本）
    background: String,         // 背景色
    cursor: String,             // 光标颜色
    cursor_accent: String,      // 光标强调色
    
    // 选择色
    selection_background: String,
    selection_foreground: String,
    selection_inactive_background: String,
    
    // ANSI 标准 8 色
    black: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    magenta: String,
    cyan: String,
    white: String,
    
    // ANSI 高亮 8 色
    bright_black: String,
    bright_red: String,
    bright_green: String,
    bright_yellow: String,
    bright_blue: String,
    bright_magenta: String,
    bright_cyan: String,
    bright_white: String,
}
```

### 14.3 主题管理

| 功能 | 说明 |
|------|------|
| 内置主题 | 预设多套亮色/暗色终端配色 |
| 自定义主题 | 用户可创建自定义配色方案 |
| 独立配置 | 亮色/暗色模式各自独立的终端配色 |
| 实时预览 | 切换主题时实时预览效果 |

### 14.4 应用界面配色

```rust
struct AppTheme {
    text_color: String,
    text_active_color: String,
    app_bg_color: String,
    app_divider_color: String,
    app_header_text_color: String,
    app_sidebar_bg_color: String,
    app_side_hover_bg_color: String,
    app_status_bar_bg_color: String,
    app_left_list_bg_color: String,
    app_content_term_bg_color: String,
    dialog_bg_color: String,
    dropdown_color: String,
    input_content_bg_color: String,
    table_bg_color: String,
}
```

### 14.5 Tab 类型颜色

不同类型的 Tab 可配置不同的颜色标识：
- 默认色、活跃色、悬停色
- 按连接类型区分（如生产环境红色、测试环境绿色）

---

## 15. 设置与偏好

### 15.1 终端设置

| 设置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| 字体族 | 下拉 | 系统等宽字体 | 终端字体 |
| 字号 | 数字 | 14 | 12-22 范围 |
| 字体加粗 | 开关 | 关 | 是否加粗 |
| 光标样式 | 选择 | block | block/underline/bar |
| 双击复制 | 开关 | 开 | 双击选词自动复制 |
| 复制分隔符 | 文本 | 空格 | 双击选词的分隔字符 |
| 右键模式 | 选择 | 快捷编辑 | 快捷编辑/标准菜单 |
| 渲染模式 | 选择 | GPU | GPU/CPU |
| SSH 空闲超时 | 数字 | 0 | 自动关闭空闲连接（秒，0=不关闭） |
| SSH 密钥 | 选择 | - | 默认使用的 SSH 密钥 |

### 15.2 通用设置

| 设置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| 主题 | 选择 | 暗色 | 亮色/暗色/跟随系统 |
| 语言 | 选择 | 中文 | 界面语言 |
| 自动更新 | 开关 | 开 | 检查更新 |
| 更新通道 | 选择 | 稳定版 | 稳定版/测试版 |

### 15.3 设置持久化

- 所有设置存储在本地 JSON 文件
- 支持通过 Gist 同步设置到云端
- 支持导入/导出设置

---

## 16. Code-Server 集成

### 16.1 连接模式

| 模式 | 说明 |
|------|------|
| 直连 | 直接连接远程 Code-Server 端口 |
| SSH 隧道 | 通过 SSH 本地端口转发连接 |
| SSH 命令 | 通过 SSH 命令启动隧道 |

### 16.2 配置

```rust
struct CodeServer {
    server_type: u8,    // 0=直连, 1=SSH隧道, 2=SSH命令
    port: u16,          // 直连端口
    range_port: String, // 隧道端口范围 "3000-9000"
}
```

---

## 17. RDP 远程桌面

### 17.1 功能

| 功能 | 说明 |
|------|------|
| RDP 连接 | 连接 Windows 远程桌面 |
| 连接管理 | 保存 RDP 连接配置 |
| RDP 导入 | 导入 .rdp 文件 |
| 进程管理 | 管理 RDP 会话进程 |

### 17.2 实现

- 调用系统 FreeRDP / mstsc 客户端
- 跨平台支持（Windows/macOS/Linux 各有实现）
- 临时 RDP 文件生成

---

## 18. 数据同步

### 18.1 Gist 同步

| 功能 | 说明 |
|------|------|
| 上传配置 | 将连接配置上传到 Gitee Gist |
| 下载配置 | 从 Gist 下载连接配置 |
| 合并策略 | 智能合并本地和云端配置 |
| 偏好同步 | 同步应用偏好设置 |

### 18.2 同步数据范围

- 连接列表和分组
- 收藏命令
- 应用偏好设置
- 终端主题配置

---

## 19. 安全特性

### 19.1 密码管理

| 特性 | 说明 |
|------|------|
| 加密存储 | AES-256-CFB 加密所有密码 |
| 硬件绑定 | 密钥基于主板序列号生成 |
| 内存安全 | 密码使用后及时清除 |
| 密码提示 | 支持密码提示字段 |

### 19.2 SSH 安全

| 特性 | 说明 |
|------|------|
| 密钥认证 | 优先推荐密钥认证 |
| 密钥生成 | 内置 ED25519 密钥生成 |
| 已知主机 | SSH known_hosts 验证 |
| 协议加密 | SSH 协议层加密所有传输 |

### 19.3 文件安全

| 特性 | 说明 |
|------|------|
| 编辑限制 | 可执行文件不可在线编辑 |
| 大小限制 | 超过 5MB 的文件不可在线编辑 |
| 权限检查 | SFTP 操作前检查文件权限 |

---

## 20. 事件与通信架构

### 20.1 前后端通信

原项目使用 Wails 事件系统，Rust 原生实现需要替代方案：

| 事件类型 | 说明 |
|---------|------|
| `tty-data:{session_id}` | 终端输出数据（Base64 编码） |
| `ssh:{event_type}` | SSH 连接状态事件 |
| `console:{event_type}` | 本地终端状态事件 |
| `ssh-tunnel-disconnected` | 隧道断开通知 |
| `sftp-progress` | 文件传输进度 |
| `sftp-error` | 文件传输错误 |

### 20.2 数据编码

- 终端输出: Base64 编码传输
- 终端输入: UTF-8 文本直接发送
- 文件内容: Base64 编码
- 配置数据: JSON 序列化

---

## 21. 路径提取与智能功能

### 21.1 路径提取器

自动从终端输出中提取当前工作目录：

| 模式 | 正则示例 |
|------|---------|
| Unix 提示符 | `user@host:/path/to/dir$` |
| Windows PS | `PS C:\Users\name>` |
| Windows CMD | `C:\Users\name>` |

用途：SFTP 打开时自动定位到终端当前目录。

### 21.2 ANSI 清理

提取路径前需清除 ANSI 转义序列，确保正则匹配准确。

---

## 22. 断线重连机制

### 22.1 重连流程

```
SSH 连接断开 → 检测到 EOF/错误
      ↓
显示断开提示信息
      ↓
用户按 Enter → 触发重连
      ↓
使用原连接配置重新建立 SSH
      ↓
重新分配 PTY → 恢复终端会话
```

### 22.2 重连特性

| 特性 | 说明 |
|------|------|
| 手动触发 | 按 Enter 键触发重连 |
| 配置复用 | 使用原始连接配置 |
| 状态重置 | 重连后清除旧终端状态 |
| 错误提示 | 显示断开原因 |

---

## 23. 免密登录配置

### 23.1 流程

```
1. 生成 ED25519 密钥对（如果不存在）
2. 读取本地公钥内容
3. 通过 SSH 连接到远程主机
4. 将公钥追加到 ~/.ssh/authorized_keys
5. 设置正确的文件权限 (600)
6. 验证免密登录是否成功
```

### 23.2 密钥管理

| 功能 | 说明 |
|------|------|
| 密钥生成 | ED25519 算法 |
| 密钥存储 | ~/.ssh/id_ed25519 |
| 密钥选择 | 支持多个密钥，可选择使用哪个 |
| 公钥部署 | 自动部署到远程 authorized_keys |
| 密钥扫描 | 扫描 ~/.ssh 目录列出所有密钥 |

---

## 24. 非功能性需求

### 24.1 性能要求

| 指标 | 要求 |
|------|------|
| 终端渲染 | 60fps 流畅渲染，大量输出不卡顿 |
| 启动时间 | < 1 秒冷启动 |
| 内存占用 | 单终端 < 50MB |
| 连接建立 | SSH 连接 < 5 秒（网络正常时） |
| 输入延迟 | < 10ms 按键到显示 |
| 大文件输出 | 4MB 缓冲 + 背压控制，不崩溃 |

### 24.2 跨平台

| 平台 | 要求 |
|------|------|
| Windows 10/11 | 完整支持 |
| macOS 12+ | 完整支持 |
| Linux (X11/Wayland) | 完整支持 |

### 24.3 可靠性

| 要求 | 说明 |
|------|------|
| 崩溃恢复 | 异常退出后可恢复会话列表 |
| 数据安全 | 密码加密存储，不明文泄露 |
| 资源清理 | 关闭时优雅释放所有 PTY/SSH 连接 |
| 并发安全 | 多终端并发操作不冲突 |

---

## 25. Rust 实现建议

### 25.1 推荐技术栈

| 组件 | 推荐方案 | 说明 |
|------|---------|------|
| GUI 框架 | iced / egui / gpui | 原生 GPU 渲染，不依赖 Webview |
| 终端仿真 | alacritty_terminal (vte) | Alacritty 的终端仿真库 |
| 文本渲染 | cosmic-text / glyphon | GPU 加速文本渲染 |
| PTY | portable-pty / rustix | 跨平台 PTY |
| SSH | russh / thrussh | 纯 Rust SSH 实现 |
| SFTP | russh-sftp | 基于 russh 的 SFTP |
| 异步运行时 | tokio | 异步 I/O |
| 序列化 | serde + serde_json | 配置序列化 |
| 加密 | aes / ring | AES-256 加密 |
| 文件监听 | notify | 文件系统事件 |
| 系统信息 | sysinfo | 本地系统信息 |
| RDP | freerdp-rs 或调用系统命令 | RDP 客户端 |

### 25.2 核心模块划分

```
src/
├── main.rs                 # 应用入口
├── app/                    # 应用框架
│   ├── mod.rs
│   ├── window.rs           # 窗口管理
│   ├── theme.rs            # 主题系统
│   └── config.rs           # 全局配置
├── terminal/               # 终端核心
│   ├── mod.rs
│   ├── emulator.rs         # VT 终端仿真
│   ├── renderer.rs         # GPU 文本渲染
│   ├── buffer.rs           # 滚动缓冲区
│   ├── selection.rs        # 文本选择
│   ├── input.rs            # 键盘输入处理
│   └── write_queue.rs      # 输出背压队列
├── pty/                    # PTY 管理
│   ├── mod.rs
│   ├── local.rs            # 本地 PTY
│   └── resize.rs           # 窗口大小同步
├── ssh/                    # SSH 模块
│   ├── mod.rs
│   ├── client.rs           # SSH 客户端
│   ├── session.rs          # SSH 会话
│   ├── auth.rs             # 认证（密码/密钥）
│   ├── tunnel.rs           # 端口转发
│   ├── proxy.rs            # 代理连接
│   └── pool.rs             # 连接池
├── sftp/                   # SFTP 模块
│   ├── mod.rs
│   ├── client.rs           # SFTP 客户端
│   ├── transfer.rs         # 文件传输（进度/取消）
│   └── operations.rs       # 文件操作
├── connection/             # 连接管理
│   ├── mod.rs
│   ├── storage.rs          # 连接持久化
│   ├── group.rs            # 分组管理
│   ├── import_export.rs    # 导入导出（JSON/XShell）
│   └── encryption.rs       # 密码加密
├── ui/                     # UI 组件
│   ├── mod.rs
│   ├── tab_bar.rs          # Tab 栏
│   ├── sidebar.rs          # 侧边栏（连接列表）
│   ├── split_pane.rs       # 分屏管理
│   ├── context_menu.rs     # 右键菜单
│   ├── toolbar.rs          # 工具栏
│   ├── monitor.rs          # 系统监控面板
│   ├── sftp_panel.rs       # SFTP 面板
│   ├── settings.rs         # 设置界面
│   └── quick_cmd.rs        # 快捷命令
├── monitor/                # 远程监控
│   ├── mod.rs
│   └── sysinfo.rs          # 系统信息采集
├── sync/                   # 数据同步
│   ├── mod.rs
│   └── gist.rs             # Gist 同步
└── utils/                  # 工具
    ├── mod.rs
    ├── path_extractor.rs   # 路径提取
    ├── machine_id.rs       # 机器标识
    └── encoding.rs         # 编码处理
```

### 25.3 关键实现注意事项

1. **终端仿真**: 推荐直接使用 `alacritty_terminal` crate，它已经实现了完整的 VT 仿真，避免从零实现
2. **GPU 渲染**: 使用 wgpu 或 OpenGL 进行文本渲染，参考 Alacritty 的渲染管线
3. **SSH 连接池**: 使用 Arc<Mutex<HashMap>> 管理连接池，同一主机复用连接
4. **异步 I/O**: 所有网络操作使用 tokio 异步执行，避免阻塞 UI
5. **背压控制**: 终端输出队列超过 4MB 时截断，防止内存溢出
6. **跨平台 PTY**: Windows 使用 ConPTY API，Unix 使用 openpty
7. **密码安全**: 密码在内存中使用 zeroize crate 确保清除
8. **事件驱动**: UI 更新采用事件驱动模型，终端输出通过 channel 推送到渲染线程

### 25.4 与原项目的关键差异

| 方面 | 原项目 (Wails) | Rust 原生 |
|------|---------------|-----------|
| 终端渲染 | xterm.js (Canvas/WebGL) | GPU 原生文本渲染 |
| UI 框架 | Vue 3 (HTML/CSS) | 原生 GUI (iced/egui/gpui) |
| 前后端通信 | Wails 事件绑定 | 直接函数调用 / channel |
| PTY | go-console | portable-pty / ConPTY |
| SSH | golang.org/x/crypto | russh |
| 性能 | 受限于 Webview | 原生性能，更低延迟 |

---

## 26. 术语表

| 术语 | 说明 |
|------|------|
| PTY | Pseudo Terminal，伪终端 |
| VT100/VT220 | DEC 终端仿真标准 |
| xterm-256color | 支持 256 色的 xterm 终端类型 |
| CSI | Control Sequence Introducer，控制序列引导符 |
| OSC | Operating System Command，操作系统命令序列 |
| SGR | Select Graphic Rendition，图形渲染选择 |
| ConPTY | Windows Console Pseudo Terminal API |
| Bracketed Paste | 括号粘贴模式，防止粘贴内容被误执行 |
| SOCKS5 | Socket Secure 5 代理协议 |
| Bastion Host | 跳板机/堡垒机 |
| ED25519 | 椭圆曲线数字签名算法 |
| AES-256-CFB | 高级加密标准 256 位密码反馈模式 |
| FreeRDP | 开源 RDP 客户端实现 |
| Code-Server | 基于 VS Code 的远程 IDE |
```
