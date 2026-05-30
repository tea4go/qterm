# WhaleTerminal 终端界面设计规范

> 本文档定义 WhaleTerminal 终端模块的完整 UI 设计规范，包括布局结构、像素尺寸、颜色来源、交互行为。供 Rust 原生程序复刻界面使用。

---

## 1. 全局布局结构

### 1.1 应用整体布局

```
┌────────────────────────────────────────────────────────────────┐
│                       标题栏 (Title Bar)                         │  ← height: titleBarHeight px
│  ┌──────┬────────────────────────────────────────┬────────────┐│
│  │ Logo │          Tab 页签栏                     │ 窗口按钮   ││
│  └──────┴────────────────────────────────────────┴────────────┘│
├──┬─────────────────────────────────────────────────────────────┤
│  │                                                             │
│  │                  内容区 (Content Area)                       │  ← flex: 1
│  │                                                             │
│  │                                                             │
├──┴─────────────────────────────────────────────────────────────┤
│                       状态栏 (Foot Bar)                          │  ← height: fontSize * rowHeightRatio px
└────────────────────────────────────────────────────────────────┘
```

### 1.2 内容区详细布局

```
┌───┬──────┬───┬───────────────────────────────────────┐
│   │      │   │                                       │
│ R │ Left │ R │           Right Pane                   │
│ i │ Pane │ s │                                       │
│ b │      │ i │                                       │
│ b │      │ z │                                       │
│ o │      │ e │                                       │
│ n │      │   │                                       │
│   │      │   │                                       │
├───┴──────┴───┼───────────────────────────────────────┤
│  左侧导航栏   │          终端/内容区域                   │
│  宽度:固定     │      背景色: appContentTermBgColor      │
│              │                                       │
│  连接列表     │  ┌──────────────────────────────────┐ │
│  (Tree)      │  │  QuickCmdBar (可选)              │ │
│              │  ├──────────────────────────────────┤ │
│              │  │                                  │ │
│              │  │     xterm 终端区域                │ │
│              │  │     padding: 2px 3px 0 5px       │ │
│              │  │                                  │ │
│              │  │                                  │ │
│              │  ├──────────────────────────────────┤ │
│              │  │  QuickCmdLine (可选)              │ │
│              │  └──────────────────────────────────┘ │
└──────────────┴───────────────────────────────────────┘
```

---

## 2. 标题栏 (Title Bar)

### 2.1 尺寸

| 属性 | 值 | 来源 |
|------|---|------|
| 高度 | `titleBarHeight` px (默认 40) | `preferenceState.config.titleBarHeight` |
| 背景色 | 动态跟随当前 Tab 的 TitleType | `titleBgColor` |
| 可拖拽区域 | 整个标题栏（非按钮区域） | `--wails-draggable: drag` |
| 文本不可选 | `user-select: none` | — |

### 2.2 Logo 区域

```
┌─────────────────────────────────────────────┐
│ 🐋 WhaleTerm  v5.8.3          [─] [□] [✕] │
│ ←leftNavWidth + leftPanelWidth+1→           │
└─────────────────────────────────────────────┘
```

| 元素 | 尺寸 | 说明 |
|------|------|------|
| Logo 图标 | 19px × 19px | `WhaleTermLogo` 组件 |
| 标题文本 | font-size: 18px, font-weight: bold, 高度 25px | "WhaleTerm" 或 "QShell" |
| 版本号 | font-size: 13px, margin-left: 8px | 圆角 5px, padding: 0 5px |
| Logo 区域宽度 | `leftNavWidth + leftPanelWidth + 1` (侧栏展开时) | — |
| macOS 左内边距 | 78px | 预留红绿灯按钮 |
| 非 macOS 左内边距 | 10px (全屏时) / 0px | — |

### 2.3 Tab 页签栏

#### 2.3.1 布局

- 使用卡片式 Tab（`type="card"`, `size="small"`, `closable`）
- Tab 栏对齐：`flex-end`（靠底部），Chat 模式下 `center`（居中）

#### 2.3.2 单个 Tab 尺寸

| 属性 | 值 |
|------|---|
| Tab 高度 | `headerHeight - 6` px (约 34px) |
| 最小宽度 | 100px |
| 最大宽度 | `tabsMaxWidth` px (默认 180) |
| 左内边距 | 5px |
| 右内边距 | 2px |
| 顶部圆角 | 8px |
| 底部圆角 | 0 (与标题栏底部对齐) |
| 右侧间距 | 1px |

#### 2.3.3 Tab 内容结构

```
┌──┬───────────────────────────┬───┐
│  │ [icon] Title...           │ ✕ │
└──┴───────────────────────────┴───┘
 ←→ 18px    ← ellipsis →     ←→6px→
  margin       n-ellipsis       margin
```

| 元素 | 样式 |
|------|------|
| 图标 | 18px，仅 SERVER/FTP 等非 BASE 类型显示 |
| 标题文本 | `word-break: break-all; white-space: nowrap; text-overflow: ellipsis` |
| 文本最小宽度 | `minWidth - 60` px |
| 文本最大宽度 | `maxWidth - 60` px |
| 文本定位 | `position: relative; top: 1px; margin-left: 5px; padding-right: 5px` |
| 关闭按钮 | margin-right: 6px, border-radius: 4px, 高度 20px, 宽度 30px（hitbox） |

#### 2.3.4 Tab 颜色状态

Tab 颜色来自连接配置的 `TabType` 或主题中的默认 Tab 类型：

| 状态 | 背景色 | 文本色 | 边框色 | 字重 |
|------|--------|--------|--------|------|
| **激活** | `tabType.activeBGColor` | `tabType.activeColor` | `tabType.activeBorderColor` | 900 |
| **未激活** | `tabType.bgColor` | `tabType.color` | `tabType.borderColor` | normal |
| **底部** | 无 (`border-bottom: unset`) | — | — | — |

#### 2.3.5 Tab 拖拽

- 拖拽指示线：蓝色虚线，宽 2px
- `drop-indicator-before`: 左侧 `border-left: 2px dashed #2080F0`
- `drop-indicator-after`: 右侧 `border-right: 2px dashed #2080F0`
- 指示线高度为 Tab 高度的 80%（top: 10%）

#### 2.3.6 Tab 溢出

当 Tab 数量过多时：
- 显示 "更多" 下拉按钮
- `ConsoleDropdown` 组件：宽 61px，高 `headerHeight - 10` px
- 顶部圆角：6px，底部无边框
- 内含 "+" 新建按钮和下拉箭头

### 2.4 窗口控制按钮 (Windows)

| 按钮 | 尺寸 | 说明 |
|------|------|------|
| 最小化/最大化/关闭 | `headerHeight × headerHeight` px | `--wails-draggable: none` |
| Hover (非关闭) | 背景色 `closeColorHover` | — |
| Hover (关闭) | 文本色 `textActiveColor` | 红色系 |
| 位置 | `align-self: flex-start` | 右上角 |

---

## 3. 左侧导航栏 (Ribbon)

### 3.1 尺寸

| 属性 | 值 | 来源 |
|------|---|------|
| 宽度 | `leftNavWidth` px | `preferenceState.config.leftNavWidth` |
| 背景色 | `appSiderBarBgColor` | 系统主题 |
| 右边框 | 1px solid `appSplitColor` | — |
| 上下内边距 | 5px | — |

### 3.2 导航按钮

| 属性 | 值 |
|------|---|
| 按钮尺寸 | `(leftNavWidth - 10) × (leftNavWidth - 10)` px |
| 内边距 | `0 5px` |
| 图标大小 | `floor(leftNavWidth × 0.4)` px |
| 字号 | `leftNavFontSize` px |
| 字重 | 700 |
| 圆角 | 8px |
| 间距 | margin-bottom: 1px |
| Hover 背景 | `appSideHoverBgColor` |
| 激活文本色 | `appSideTextActiveColor` |
| 激活背景 | `appSideHoverBgColor` |
| 未激活文本色 | `appSideTextColor` |

### 3.3 底部按钮

| 属性 | 值 |
|------|---|
| 按钮尺寸 | 30px × 30px |
| 圆角 | 4px |
| 间距 | gap: 2px |
| 包含 | 关于、主题切换、云同步、设置 |

---

## 4. 左侧面板 (Left Pane)

### 4.1 尺寸

| 属性 | 值 | 来源 |
|------|---|------|
| 宽度 | 可拖拽调整，初始值按模块不同 | 见下表 |
| 最小宽度 | 各模块默认值 | `preferenceState.defaultConfig` |
| 最大宽度 | `窗口宽度 - 500` | 确保右侧面板至少 500px |
| 背景色 | `appLeftListBgColor` | 系统主题 |
| 右边框 | 1px solid `appSplitColor` | — |

各模块默认宽度：

| 模块 | 配置键 |
|------|--------|
| 本地终端 | `leftHostWidth` |
| 研发云 | `leftWhaleZCMWidth` |
| 项目云 | `leftPrjZCMWidth` |
| ZMC | `leftPrjZmcWidth` |
| 笔记 | `leftNoteWidth` |
| Chat | `leftChatWidth` |
| 翻译 | `leftTransWidth` |

### 4.2 分割线 (Resize Divider)

| 属性 | 值 |
|------|---|
| 光标 | `col-resize` |
| 线宽 | 2px (hover/drag 时 3px) |
| 热区宽 | 4px |
| 热区高 | 20px |
| 热区圆角 | 3px |
| 默认状态 | 透明 (opacity: 0) |
| Hover | `opacity: 1`, 线色 `borderColor` |
| 拖拽中 | 线色 `textActiveColor` |
| 过渡动画 | 0.15s ease opacity |

### 4.3 连接树 (ConnectionTree)

| 属性 | 值 |
|------|---|
| 组件 | `n-tree` + `virtual-scroll` |
| 行高 | `fontSize × 3 - 4` px (字号14时为38px) |
| 支持拖拽 | `draggable` |
| 展开策略 | `expand-on-dragenter` |
| 文本溢出 | `white-space: nowrap; overflow: hidden; text-overflow: ellipsis` |
| 文本定位 | `position: relative; top: 1px` |

### 4.4 底部工具栏

| 属性 | 值 |
|------|---|
| 上边框 | 1px solid `appSplitColor` |
| 背景色 | `appLeftListBgColor` |
| 图标大小 | `(fontSize × 4) / 3` px |
| 内容 | 过滤输入框 + 导入/导出下拉 |

---

## 5. 右侧终端区域 (Right Pane)

### 5.1 容器

| 属性 | 值 |
|------|---|
| flex | 1 (占满剩余空间) |
| 最小高度 | 0 |
| 溢出 | hidden |
| 背景色 | `appLeftListBgColor` |
| 上边框 | 1px solid `appSplitColor` (仅笔记/Chat/翻译/RDP 模式) |

### 5.2 终端容器 (ServerItem)

| 属性 | 值 |
|------|---|
| 高度 | 100% |
| 背景色 | `appContentTermBgColor` |
| 显示 | 仅当前激活 Tab 可见 (`v-show`) |
| 溢出 | hidden |

### 5.3 xterm 终端容器

| 属性 | 值 |
|------|---|
| 宽度 | `calc(100% - 8px)` |
| 高度 | `calc(100% - 5px)` |
| 内边距 | `2px 3px 0 5px` (上 右 下 左) |
| 用户选择 | 禁止 (`-webkit-user-select: none`) |
| 滚动条 | 默认隐藏 (`visibility: hidden`)，hover 时显示 |

### 5.4 分屏终端

#### 5.4.1 SplitServers 容器

| 属性 | 值 |
|------|---|
| 宽度 | 100%（Monitor 打开时缩小） |
| 高度 | 100% |
| 定位 | `position: relative` |

Monitor 打开时宽度：

| Monitor 尺寸 | 容器宽度 |
|-------------|---------|
| Small | `calc(100% - 607px)` |
| Large | `calc(100% - 50vw - 7px)` |

#### 5.4.2 分屏面板定位

- 每个面板使用绝对定位 + 百分比布局
- 通过 `termLayout` 树递归计算 left/top/width/height
- 面板溢出 2px 以覆盖分割线缝隙
- 最大分屏数：6

#### 5.4.3 分割线 (Splitter)

| 属性 | 值 |
|------|---|
| 厚度 | 6px |
| 最小面板宽度 | 145px |
| 最小面板高度 | 120px |
| 层级 | z-index: 100 |
| 圆角 | 2px |
| 背景色 | `textActiveColor` |
| 光标 (水平) | `col-resize` |
| 光标 (垂直) | `row-resize` |
| Hover | 分割线展开至 100%（正常为 `calc(100% - 4px)`） |
| 过渡 | background-color 0.3s ease-in |

---

## 6. 快捷命令栏 (QuickCmdBar)

位于终端上方，显示收藏命令按钮。

```
┌──────────────────────────────────────────────┐
│ [cmd1] [cmd2] [cmd3] ...          [⏹ Stop] │
└──────────────────────────────────────────────┘
```

| 属性 | 说明 |
|------|------|
| 位置 | 终端区域上方 |
| 显示/隐藏 | 每个 Tab 独立控制 |
| 按钮 | 显示 `quick: true` 的收藏命令 |
| 发送方式 | 逐行发送，每行间隔 100ms |
| 拖拽 | 支持按钮拖拽排序 |
| 空状态 | 提示 "添加收藏命令" 链接 |
| 停止按钮 | 发送 Ctrl+C + Y 中断命令 |

---

## 7. 快捷命令输入框 (QuickCmdLine)

位于终端下方的可调整高度的命令输入区域。

```
┌──────────────────────────────────────────────┐
│ [反斜杠] [逐行] [Enter] │ textarea... │ [发送]│
│ ↑ 拖拽手柄调整高度                              │
└──────────────────────────────────────────────┘
```

| 属性 | 说明 |
|------|------|
| 位置 | 终端区域下方 |
| 样式 | 匹配当前终端字体（大小、粗细）和主题颜色（前景/背景） |
| 最小高度 | 单行文本 |
| 最大高度 | 70vh |
| 高度调整 | 顶部拖拽手柄 (`row-resize` 光标) |
| 展开/折叠 | 按钮切换最小/最大高度 |
| 持久化 | 命令文本按 Tab 独立保存 |

三个切换按钮（左侧）：

| 按钮 | 默认 | 说明 |
|------|------|------|
| 反斜杠转换 | 关 | 非标准 `\x` 转为 `\\x` |
| 发送模式 | 逐行 | 逐行发送(带`\r`) / 合并发送(空格连接) |
| 发送触发 | Enter | Enter 发送 / Ctrl+Enter 发送 |

---

## 8. 系统监控面板 (Monitor)

位于终端区域右侧的抽屉式面板。

### 8.1 面板尺寸

| 尺寸 | 宽度 |
|------|------|
| Small | 600px |
| Large | 50vw (50% 窗口宽度) |
| z-index | 1 |

### 8.2 面板内容

```
┌──────────────────────────────────┐
│ sysinfo v1.0  [S/L] [刷新] [✕] │  ← 头部
├──────────────────────────────────┤
│ CPU  ██████░░░░  65%            │
│ 内存 ████████░░  80%            │
│ 磁盘 ██░░░░░░░░  20%            │
│ 网络 ▲ 1.2MB/s  ▼ 3.4MB/s      │
│                                  │
│ PID   USER  CPU%  MEM%  CMD     │  ← 进程表
│ 1234  root  12.3  5.6   nginx   │
│ ...                              │
└──────────────────────────────────┘
```

| 元素 | 说明 |
|------|------|
| 头部 | sysinfo 版本 + 尺寸切换器 + 刷新按钮 + 关闭按钮 |
| 内容 | `n-scrollbar` 滚动包裹 `MonitorInfo` 组件 |
| 刷新 | 通过 SSH 执行远程 sysinfo 命令采集 |
| 自动启动 | 可配置连接后自动打开 |

---

## 9. 状态栏 (Foot Bar)

### 9.1 尺寸

| 属性 | 值 | 来源 |
|------|---|------|
| 高度 | `fontSize × rowHeightRatio` px | `preferenceState.config` |
| 内边距 | `0 9px` | — |
| 上边框 | 1px solid `appSplitColor` | — |
| 背景色 | 动态跟随 TitleType | `statusBarBgColor` |
| 文本色 | 动态跟随 TitleType | `statusBarTextColor` |

### 9.2 左侧区域

```
[●] session-name | extra-info
```

| 元素 | 说明 |
|------|------|
| 连接状态点 | 圆形，直径 `footerHeight/3` px，圆角 `footerHeight/6` px |
| 绿色 | 已连接 |
| 红色 | 未连接 |
| 会话名称 | 当前 Tab 标题 |
| 分隔符 | `\|` 竖线 |
| 额外信息 | 版本号 / FTP 选中信息 / 模块状态 |

### 9.3 右侧区域

最小宽度：220px (有主机连接按钮时) / 170px / 80px

```
[按键记录] [Tab历史] | [收藏命令] [快捷栏] [快捷行] [监控] | [知识库] [AI] [翻译]
```

| 区域 | 内容 |
|------|------|
| 终端操作 | 按键记录、Tab 历史 |
| 终端提示 | 收藏命令、快捷命令栏、快捷命令行、监控面板 |
| 模块切换 | 知识库、AI Chat、翻译 |

各按钮间分隔：`border-left: 1px solid appDividerColor`, `padding-left: 8px`, `margin-left: 8px`

| 属性 | 值 |
|------|---|
| 字号 | `defaultFontSize + 2` px |
| 图标大小 | 默认尺寸 + 2px |
| 文本色 | `statusBarTextColor` |
| Hover 文本色 | `appStatusBarTextHoverColor` |

---

## 10. 右键上下文菜单

### 10.1 菜单类型

有两种右键模式，可在设置中切换：

| 模式 | 触发条件 | 行为 |
|------|---------|------|
| **标准菜单** | 右键 | 弹出完整菜单 |
| **快捷编辑** | 右键 + 有选中文本 | 自动复制选中文本 + 如果剪贴板有内容则粘贴 |

### 10.2 终端右键菜单项

| 菜单项 | 功能 | 条件 |
|--------|------|------|
| 复制 | 复制选中文本 | — |
| 粘贴 | 粘贴剪贴板内容 | — |
| 粘贴选中 | 粘贴当前选中文本 | 有选中文本时 |
| 清屏 | 清除终端 (cls) | — |
| 全选 | 选中所有终端内容 | — |
| 重新连接 | 断开后重连 | — |
| 设置终端宽度 | 调整终端列数 | — |
| 分屏-水平 | 水平分割 | 子终端 < 6 |
| 分屏-垂直 | 垂直分割 | 子终端 < 6 |
| --- | 分隔线 | — |
| 收藏命令 | 显示收藏命令 | — |
| 快捷命令栏 | 显示/隐藏快捷栏 | — |
| 快捷命令行 | 显示/隐藏命令行 | — |
| 主机信息 | 显示/隐藏监控 | 远程连接 |
| --- | 分隔线 | — |
| 安装工具 | 安装远程监控工具 | 远程连接 |
| Code-Server | 启动远程 IDE | 配置了 Code-Server |
| SFTP | 打开 SFTP 文件管理 | 远程连接 |
| 免密登录 | 配置 SSH 密钥 | 远程连接 |

### 10.3 Tab 右键菜单项

| 菜单项 | 功能 |
|--------|------|
| 新建连接 | 打开新连接 |
| 复制连接 | 复制当前连接配置 |
| 重命名 | 修改 Tab 标题 |
| 颜色标记 | 设置 Tab 颜色 |
| 分隔线 | — |
| 关闭 | 关闭当前 Tab |
| 关闭左侧 | 关闭左侧所有 Tab |
| 关闭右侧 | 关闭右侧所有 Tab |

---

## 11. 交互行为

### 11.1 键盘交互

| 操作 | 行为 |
|------|------|
| Ctrl+C (有选中文本) | 复制选中文本 |
| Ctrl+C (无选中文本) | 发送 SIGINT |
| Ctrl+V | 粘贴到终端 |
| Enter (有选中文本) | 复制选中文本并清除选择 |
| 双击 | 选中一个单词（使用配置的分隔符） |
| Ctrl+= | 字体放大 (最大 30) |
| Ctrl+- | 字体缩小 (最小 11) |
| F6-F12 | 可配置的自定义快捷键（发送 ESC 序列或自定义文本） |

### 11.2 鼠标交互

| 操作 | 行为 |
|------|------|
| 左键拖拽 | 选择文本 |
| 双击左键 | 选中单词并自动复制（可配置关闭） |
| 三击左键 | 选中整行 |
| 滚轮 | 终端内容滚动 |
| 右键 | 弹出上下文菜单（或快捷编辑模式） |
| 右键 + 拖拽 | 无 |

### 11.3 Tab 交互

| 操作 | 行为 |
|------|------|
| 左键单击 | 激活 Tab |
| 左键拖拽 | 重新排序 |
| 双击 | 复制连接 |
| 右键 | 弹出 Tab 菜单 |
| 中键点击 | 关闭 Tab |
| 点击终端区域 | 切换到该分屏 (switchChildTab) |

### 11.4 分屏导航

| 操作 | 说明 |
|------|------|
| 方向键导航 | `switchChildTabByDirection` 通过 termLayout 树查找相邻面板 |
| 顺序导航 | `paneNavNext` / `paneNavPrevious` 循环切换 |

### 11.5 断线重连

```
连接断开 → 显示断开提示 → 用户按 Enter → 触发重连 → 使用原配置重建连接
```

- 断开时终端显示警告信息
- 设置 `reconnect` 标志
- Enter 键触发 `onReloadTerm()`
- 重连后清除 `reconnect` 和 `enterClick` 标志

### 11.6 终端字体大小

通过 `Ctrl+=` / `Ctrl+-` 调整当前终端的字体大小（不影响全局设置），范围 11-30。

---

## 12. 标题栏/侧边栏/状态栏联动

当终端 Tab 激活时，三个栏的颜色同步更新：

```
Tab 激活 → 读取连接的 titleType → 提取颜色
         → 标题栏背景/文本色更新
         → bus.emit('update-title-type', bgColor, color)
         → Ribbon 侧边栏背景/文本色更新
         → 状态栏背景/文本色更新
```

这确保整个应用边框与当前连接类型视觉统一。

---

## 13. 颜色映射总结

### 13.1 各区域使用的主题色

| 区域 | 背景色 | 文本色 | 其他 |
|------|--------|--------|------|
| 标题栏 | `titleBgColor` (动态) | `titleColor` (动态) | — |
| 侧边栏 | `appSiderBarBgColor` | `appSideTextColor` | hover: `appSideHoverBgColor` |
| 左侧列表 | `appLeftListBgColor` | `textColor` | 选中: `appLeftListBgColorActive` |
| 终端区域 | `appContentTermBgColor` | 终端前景色 (xterm) | — |
| 弹框 | `dialogBgColor` | `dialogTextColor` | border: `dialogBorderColor` |
| 下拉菜单 | `dropDownBgColor` | `dropDownColor` | 选中: `dropDownActiveColor` |
| 状态栏 | `statusBarBgColor` (动态) | `statusBarTextColor` (动态) | hover: `statusBarTextHoverColor` |
| 分割线 | `appSplitColor` | — | — |
| 小分割线 | `appDividerColor` | — | — |
| 组件边框 | — | — | `borderColor` / `inputContentBorderColor` |

### 13.2 扩展色使用

| 元素 | 暗色值 | 亮色值 |
|------|--------|--------|
| Tab 激活文本 | `#FFFFFF` | `#3599FF` |
| 图标色 | `#CCCCCC` | `#000000` |
| 激活色 | `#FFFFFF` | `#007ACC` |
| 搜索图标 | `#CCCCCB` | `#7C868F` |
| 已连接状态点 | `#12A2C5` | `#7EADE2` |
| 大纲 hover | `#FFFFFF` | `#4285F4` |
| 笔记工具栏 | `#023848` | `#F5F5F5` |
| 搜索结果数背景 | `#015367` | `#D8D8D8` |

---

## 14. 非颜色布局常量

| 常量 | 默认值 | 说明 |
|------|--------|------|
| `titleBarHeight` | 40 | 标题栏高度 |
| `leftNavWidth` | ~50 | 侧边栏宽度 |
| `leftNavFontSize` | 13 | 侧边栏字体大小 |
| `defaultFontSize` | 13 | 全局默认字号 |
| `rowHeightRatio` | 2 | 行高系数 (行高 = fontSize × 2) |
| `tabsMaxWidth` | 180 | Tab 最大宽度 |
| `menuMaxWidth` | 250 | 菜单最大宽度 |
| `menuBorderRadius` | 6 | 菜单圆角 |
| `scrollbarWidth` | 8 | 滚动条宽度 |
| `borderRadius` | 8 | 组件圆角 |
| `borderRadiusSmall` | 4 | 小组件圆角 |
| `defaultFontFamily` | "Microsoft YaHei Mono, Microsoft YaHei" | 默认字体 |
| `disModalAnimation` | true | 禁用模态框动画 |

---

## 15. Rust 实现建议

### 15.1 布局引擎

```
┌─ Window ──────────────────────────────────┐
│ ┌─ TitleBar ────────────────────────────┐ │
│ │ Logo │  TabStrip (scrollable)  │ WinCtrl│ │
│ └───────────────────────────────────────┘ │
│ ┌─ Content (row) ───────────────────────┐ │
│ │ ┌Ribbon┐ ┌LeftPane┐ ┌─ RightPane ──┐ │ │
│ │ │ Icon │ │ Tree   │ │ QuickCmdBar  │ │ │
│ │ │ Nav  │ │ +      │ │ xterm area   │ │ │
│ │ │      │ │ Resize │ │ QuickCmdLine │ │ │
│ │ │      │ │ Divider│ │              │ │ │
│ │ └──────┘ └────────┘ └──────────────┘ │ │
│ └───────────────────────────────────────┘ │
│ ┌─ FootBar ─────────────────────────────┐ │
│ │ Status dot │ Session info  │ Buttons  │ │
│ └───────────────────────────────────────┘ │
└───────────────────────────────────────────┘
```

- 每个区域作为独立的 `Widget`/`Element`
- 左侧面板宽度通过拖拽 divider 实时调整，保存到配置
- 分屏通过递归 tree 结构 + 百分比定位实现
- 使用 `flex-grow` 语义确保自适应

### 15.2 关键 UI 框架选择

| 组件 | 推荐 |
|------|------|
| 布局引擎 | iced layout / egui layout |
| Tab 组件 | 自定义绘制（Card 样式 + 拖拽排序） |
| Tree 组件 | 自定义或 egui Tree |
| 滚动容器 | 内置虚拟滚动 |
| 右键菜单 | 原生弹出或自定义绘制 |
| 拖拽分割线 | 全局 mousemove 监听 + 热区检测 |
| 窗口控制按钮 | 平台原生或自定义绘制 |

### 15.3 SFTP 文件管理面板

从终端右键菜单打开，双面板布局：

```
┌─────────────────────┬─────────────────────┐
│    本地文件面板      │    远程文件面板      │
│                     │                     │
│ 路径栏 / 导航       │ 路径栏 / 导航       │
│ ┌─────────────────┐ │ ┌─────────────────┐ │
│ │ 📁 documents    │ │ │ 📁 html         │ │
│ │ 📁 downloads    │ │ │ 📁 logs         │ │
│ │ 📄 config.yml   │ │ │ 📄 index.html   │ │
│ └─────────────────┘ │ └─────────────────┘ │
│                     │                     │
│ 工具栏 / 状态栏      │ 工具栏 / 状态栏      │
└─────────────────────┴─────────────────────┘
│              传输任务列表 / 进度条           │
└────────────────────────────────────────────┘
```

| 元素 | 说明 |
|------|------|
| 位置 | 独立 Tab 或侧边抽屉 |
| 双面板 | 左侧本地、右侧远程 |
| 文件列表 | 支持树形/列表视图、拖拽上传下载 |
| 路径栏 | 当前目录路径 + 导航按钮（返回/上级/刷新） |
| 工具栏 | 新建/删除/重命名/复制/移动/权限/搜索 |
| 状态栏 | 选中文件数/总大小 |
| 冲突处理 | 同名文件弹窗选择（覆盖/重命名/跳过/确认） |

### 15.4 连接管理面板

#### 表单布局

连接新建/编辑使用表单弹窗，分为以下 Tab 页签：

| 页签 | 内容 |
|------|------|
| 基础信息 | 名称、地址、端口、用户名、描述 |
| 认证方式 | 密码 / 私钥选择、密钥管理 |
| 终端设置 | 启动命令、默认路径、快速命令行 |
| 代理 | 无代理 / 系统代理 / 自定义代理 |
| 隧道 | 本地端口转发 / 远程端口转发 / 动态转发 |
| Code-Server | 直连 / SSH 隧道 / SSH 命令模式 |
| 系统信息 | 自动监控开关、刷新间隔 |

| 元素 | 说明 |
|------|------|
| 测试连接 | 独立按钮，显示连接结果（成功/失败/耗时） |
| 密码输入 | 密码框 + 密码提示字段 + 显示/隐藏切换 |
| 密钥选择 | 下拉列表 + 生成新密钥按钮（ED25519） |
| 分组管理 | 分组选择器 + 新建分组 |
| 代理类型 | Radio 选择：无 / 系统 / 自定义 |
| 自定义代理 | Schema (HTTP/SOCKS5) + 地址 + 端口 + 认证 |
| 端口转发 | 列表式增删，每行：本地 IP:Port → 远程 IP:Port |

### 15.5 额外主题色速查表（Extra Theme）

这些颜色硬编码在 `extra_theme.js` 中，根据 light/dark 模式选择，不在主题配置中：

**终端相关：**

| 色值键 | 暗色 | 亮色 | 用途 |
|--------|------|------|------|
| `tabActiveTextColor` | `#FFFFFF` | `#3599FF` | 终端 Tab 选中文本、分屏标题 |
| `tabIconColor` | `#CCCCCC` | `#000000` | Tab 图标 |
| `activeColor` | `#FFFFFF` | `#007ACC` | 通用激活色 |
| `searchIconColor` | `#CCCCCB` | `#7C868F` | 搜索图标 |
| `termConnectedColor` | `#12A2C5` | `#7EADE2` | 已连接状态点 |
| `editDisabledColor` | `#FFFFFF61` | `#C2C2C2` | 编辑禁用态 |

**笔记工具栏相关：**

| 色值键 | 暗色 | 亮色 | 用途 |
|--------|------|------|------|
| `noteTabHeaderBorder` | `#105C5D` | `#D9D9D9` | 笔记左侧 Tab 底部边框 |
| `noteToolBarHeaderBg` | `#023848` | `#F5F5F5` | Markdown 工具栏背景 |
| `noteSearchNumBg` | `#015367` | `#D8D8D8` | 搜索结果数字背景 |
| `outlineHoverColor` | `#FFFFFF` | `#4285F4` | 大纲 hover 颜色 |

**SFTP 进度条：**

| 色值键 | 暗色 | 亮色 |
|--------|------|------|
| `ftpProgressColor` | `#005A6F` | `#34AB26` |
| `ftpProgressBorderColor` | `#1A7778` | `#C9C9C9` |
| `ftpProgressTextColor` | `#CCCCCC` | `#FFFFFF` |
| `ftpProgressRailColor` | `#00404E` | `#D9D9D9` |

**系统监控表格：**

| 色值键 | 暗色 | 亮色 |
|--------|------|------|
| `tableThBg` | `#053747` | `#EBEBEB` |
| `tableTdBg` | `#00303F` | `#F9F9F9` |
| `tableHoverColor` | `#033C4F` | `#EEEEEE` |
| `expandTableBg` | `#002733` | `#FFFFFF` |

### 15.6 窗口默认尺寸

| 属性 | 值 | 说明 |
|------|---|------|
| 默认宽度 | `windowWidth` | 保存到 `behavior.windowWidth` |
| 默认高度 | `windowHeight` | 保存到 `behavior.windowHeight` |
| 最大化状态 | `windowMaximised` | 保存到 `behavior.windowMaximised` |
| 窗口位置 X | `windowPosX` | 保存到 `behavior.windowPosX` |
| 窗口位置 Y | `windowPosY` | 保存到 `behavior.windowPosY` |

所有窗口尺寸和位置在关闭时持久化，下次启动恢复。

### 15.7 本地终端启动面板

本地终端启动时展示的"开始连接"提示：

| 元素 | 说明 |
|------|------|
| 触发条件 | 右侧面板无任何 Tab 时显示 |
| 组件 | `CreateConnectTip` |
| 内容 | "新建连接" 按钮 / 最近连接列表 / 本地终端快捷入口 |
| 背景色 | `appContentTermBgColor` |

### 15.8 分屏模式下的子终端头部

当存在分屏时，每个子终端顶部显示简要信息栏：

```
┌──────────────────────────────────────────────┐
│ host@192.168.1.1     [水平分屏] [垂直] [关闭] │  ← padding: 2px 2px 0 5px
├──────────────────────────────────────────────┤
│                 xterm 区域                    │
└──────────────────────────────────────────────┘
```

| 元素 | 说明 |
|------|------|
| 左侧 | 连接信息（主机名/IP） |
| 右侧 | 水平分屏 / 垂直分屏 / 关闭按钮，宽 60px |
| 按钮大小 | 图标大小 `(fontSize × 4) / 3` |
| 活跃指示 | `isMulti` 模式下，选中终端有焦点边框 |

### 15.9 快捷键提示组件 (TermPressKey)

位于状态栏右侧，显示当前可输入的组合键状态（如 Ctrl、Shift、Alt 的按下状态）。

### 15.10 Tab 历史菜单 (TabHistory)

位于状态栏右侧，下拉菜单显示最近关闭的 Tab 列表，支持快速重新打开。

### 15.11 无连接时的空状态

| 状态 | 显示内容 |
|------|---------|
| 无 Tab | `CreateConnectTip` 组件（新建连接 + 最近列表） |
| 连接中 | Loading 动画 + 连接地址信息 |
| 连接失败 | 错误信息 + 重试按钮 |
| 断线 | "按 Enter 重新连接" 提示 |

### 15.7 响应式设计

所有尺寸建议用相对值（字号倍数），保持与 `defaultFontSize` 和 `rowHeightRatio` 的关系，便于主题切换时全局调整。
