# WhaleTerminal 主题系统设计规范

> 本文档定义 WhaleTerminal 的完整主题系统，用于 Rust 原生程序保持与 WhaleTerminal 一致的界面风格。所有色值均从源码精确提取。

---

## 1. 主题系统架构

### 1.1 三大主题子系统

| 子系统 | 用途 | 独立亮/暗色 |
|--------|------|-------------|
| 系统主题 (SystemTheme) | 应用窗口、侧边栏、弹框、表格、按钮等 UI 组件 | 是 |
| 终端主题 (TerminalTheme) | 终端仿真器（VT100/xterm-256color）配色 | 是 |
| 笔记主题 (NoteTheme) | Markdown 编辑器（代码块、引用、表格、标题） | 是 |

### 1.2 主题模式

| 模式 | 值 | 说明 |
|------|---|------|
| 亮色 | `light` | 使用 Light 主题 |
| 暗色 | `dark` | 使用 Dark 主题 |
| 跟随系统 | `auto` | 跟随 OS 暗色模式自动切换 |

### 1.3 主题管理

- 每个子系统提供预设主题列表（Default）和用户自建主题列表（Custom）
- 当前使用的主题存在 `ThemeLight` / `ThemeDark` 字段
- 切换亮/暗模式时，自动加载对应的 Light/Dark 主题
- 主题按 `themeName` 唯一���识

---

## 2. 系统主题 (SystemTheme)

### 2.1 数据结构 (Rust)

```rust
struct SystemTheme {
    theme_name: String,
    originate: String,  // 来源主题名，用于"恢复默认"功能

    // === 应用基础 ===
    text_color: String,               // 全局默认前景色
    text_active_color: String,        // 主色调（激活色）
    app_bg_color: String,             // 应用头部背景、分隔线、表格头部背景
    app_divider_color: String,        // 所有 divider 分割线
    app_split_color: String,          // 大模块分割线
    border_color: String,             // Input/Select 组件边框

    // === 应用头部 ===
    app_header_text_color: String,

    // === 侧边栏 ===
    app_sider_bar_bg_color: String,       // 左侧边栏背景
    app_side_hover_bg_color: String,      // 侧边栏按钮 hover/选中背景
    app_side_text_active_color: String,   // 侧边栏按钮激活文本
    app_side_text_color: String,          // 侧边栏按钮文本

    // === 状态栏 ===
    app_status_bar_bg_color: String,
    app_status_bar_text_color: String,
    app_status_bar_text_hover_color: String,

    // === 左侧列表 ===
    app_left_list_bg_color: String,          // 左侧菜单背景
    app_left_list_bg_color_hover: String,    // 二级菜单 hover 背景
    app_left_list_bg_color_active: String,   // 叶子节点选中颜色
    app_left_list_text_color_active: String, // 左侧列表选中文本
    app_search_title_bg_color: String,       // 搜索结果标题背景

    // === 右侧内容区域 ===
    app_content_term_bg_color: String,       // 终端主内容背景
    app_content_note_bg_color: String,       // 笔记主内容背景
    app_content_chat_bg_color: String,       // AI Chat 主内容背景
    app_content_chat_divider_color: String,  // AI Chat 分割线
    app_content_tran_bg_color: String,       // 翻译主内容背景

    // === 弹出层 (Modal/Dialog/Message/Notification) ===
    dialog_bg_color: String,
    dialog_border_color: String,
    dialog_divider_color: String,
    dialog_text_color: String,
    dialog_text_active_color: String,

    // === 下拉菜单 ===
    drop_down_color: String,            // 下拉菜单文本
    drop_down_bg_color: String,         // 下拉菜单背景
    drop_down_active_color: String,     // 下拉菜单选中文本
    drop_down_active_bg_color: String,  // 下拉菜单选中背景

    // === AI Chat 消息 ===
    app_content_chat_send_bg_color: String,       // 用户消息背景
    app_content_chat_send_border_color: String,   // 用户消息边框
    app_content_chat_reply_bg_color: String,      // AI 回复背景
    app_content_chat_reply_border_color: String,  // AI 回复边框

    // === 输入框 ===
    input_content_bg_color: String,
    input_content_border_color: String,

    // === 表格 ===
    table_bg_color: String,
    table_border_color: String,
    table_header_bg_color: String,
    table_even_row_bg_color: String,

    // === 终端 Tab 页签 ===
    terminal_tab_types: TabType,
}

struct TabType {
    name_en: String,
    color: String,             // 未激活 Tab 文本
    active_color: String,      // 激活 Tab 文本
    bg_color: String,          // 未激活 Tab 背景
    active_bg_color: String,   // 激活 Tab 背景
    border_color: String,      // 未激活 Tab 边框
    active_border_color: String, // 激活 Tab 边框
}
```

### 2.2 默认暗色主题色值

默认暗色主题名称: **"Solarized Dark"**

```
# === 应用基础 ===
text_color              = "#FFFFFF"      // 白色全局文本
text_active_color       = "#FFFFFF"      // 主色调白色
app_bg_color            = "#002B36"      // Solarized Dark 深蓝背景
app_divider_color       = "#073642"      // 分割线
app_split_color         = "#073642"      // 模块分割线
border_color            = "#1A7778"      // 组件边框

# === 头部 ===
app_header_text_color   = "#FFFFFF"

# === 侧边栏 ===
app_sider_bar_bg_color      = "#073642"
app_side_hover_bg_color     = "#073642"
app_side_text_active_color  = "#FFFFFF"
app_side_text_color         = "#CCCCCC"

# === 状态栏 ===
app_status_bar_bg_color         = "#002B36"
app_status_bar_text_color       = "#CCCCCC"
app_status_bar_text_hover_color = "#FFFFFF"

# === 左侧列表 ===
app_left_list_bg_color          = "#073642"
app_left_list_bg_color_hover    = "#09495E"
app_left_list_bg_color_active   = "#094771"
app_left_list_text_color_active = "#FFFFFF"
app_search_title_bg_color       = "#073642"

# === 右侧内容 ===
app_content_term_bg_color      = "#000000"   // 终端纯黑
app_content_note_bg_color      = "#002B36"
app_content_chat_bg_color      = "#002B36"
app_content_chat_divider_color = "#073642"
app_content_tran_bg_color      = "#002B36"

# === 弹出层 ===
dialog_bg_color          = "#00222B"
dialog_border_color      = "#1A7778"
dialog_divider_color     = "#073642"
dialog_text_color        = "#CCCCCC"
dialog_text_active_color = "#FFFFFF"

# === 下拉菜单 ===
drop_down_color          = "#CCCCCC"
drop_down_bg_color       = "#002B36"
drop_down_active_color   = "#FFFFFF"
drop_down_active_bg_color= "#09495E"

# === AI Chat ===
app_content_chat_send_bg_color       = "#073642"
app_content_chat_send_border_color   = "#1A7778"
app_content_chat_reply_bg_color      = "#00222B"
app_content_chat_reply_border_color  = "#1A7778"

# === 输入框 ===
input_content_bg_color     = "#00222B"
input_content_border_color = "#1A7778"

# === 表格 ===
table_bg_color         = "#002B36"
table_border_color     = "#073642"
table_header_bg_color  = "#073642"
table_even_row_bg_color= "#00222B"
```

### 2.3 默认亮色主题色值

默认亮色主题名称: **"Default Light Modern"**

```
# === 应用基础 ===
text_color              = "#333333"
text_active_color       = "#007ACC"
app_bg_color            = "#F5F5F5"
app_divider_color       = "#E0E0E0"
app_split_color         = "#E0E0E0"
border_color            = "#CCCCCC"

# === 头部 ===
app_header_text_color   = "#333333"

# === 侧边栏 ===
app_sider_bar_bg_color      = "#FFFFFF"
app_side_hover_bg_color     = "#E3F2FD"
app_side_text_active_color  = "#007ACC"
app_side_text_color         = "#666666"

# === 状态栏 ===
app_status_bar_bg_color         = "#F5F5F5"
app_status_bar_text_color       = "#666666"
app_status_bar_text_hover_color = "#333333"

# === 左侧列表 ===
app_left_list_bg_color          = "#FFFFFF"
app_left_list_bg_color_hover    = "#F5F5F5"
app_left_list_bg_color_active   = "#E3F2FD"
app_left_list_text_color_active = "#007ACC"
app_search_title_bg_color       = "#F5F5F5"

# === 右侧内容 ===
app_content_term_bg_color      = "#FFFFFF"
app_content_note_bg_color      = "#FFFFFF"
app_content_chat_bg_color      = "#FFFFFF"
app_content_chat_divider_color = "#E0E0E0"
app_content_tran_bg_color      = "#FFFFFF"

# === 弹出层 ===
dialog_bg_color          = "#FFFFFF"
dialog_border_color      = "#E0E0E0"
dialog_divider_color     = "#E0E0E0"
dialog_text_color        = "#333333"
dialog_text_active_color = "#007ACC"

# === 下拉菜单 ===
drop_down_color          = "#333333"
drop_down_bg_color       = "#FFFFFF"
drop_down_active_color   = "#007ACC"
drop_down_active_bg_color= "#E3F2FD"

# === AI Chat ===
app_content_chat_send_bg_color       = "#E3F2FD"
app_content_chat_send_border_color   = "#CCCCCC"
app_content_chat_reply_bg_color      = "#FFFFFF"
app_content_chat_reply_border_color  = "#CCCCCC"

# === 输入框 ===
input_content_bg_color     = "#FFFFFF"
input_content_border_color = "#CCCCCC"

# === 表格 ===
table_bg_color         = "#FFFFFF"
table_border_color     = "#E0E0E0"
table_header_bg_color  = "#F5F5F5"
table_even_row_bg_color= "#F9F9F9"
```

---

## 3. 终端主题 (TerminalTheme)

### 3.1 数据结构 (Rust)

```rust
struct TerminalTheme {
    theme_name: String,
    originate: String,

    // === 主色块 ===
    background: String,                   // 终端窗口背景
    foreground: String,                   // 默认文本颜色
    cursor: String,                       // 光标颜色
    cursor_accent: String,                // 光标覆盖文字颜色
    selection_background: String,         // 选中文本背景
    selection_foreground: String,         // 选中文本前景
    selection_inactive_background: String, // 非活动窗口选中背景

    // === ANSI 标准 8 色 ===
    black: String,
    red: String,           // 错误信息、删除线
    green: String,         // 成功信息、目录名
    yellow: String,        // 警告信息、可执行文件
    blue: String,          // 链接文件
    magenta: String,       // 图片文件、特殊文件
    cyan: String,          // 源代码文件
    white: String,

    // === ANSI 高亮 8 色 ===
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

### 3.2 默认暗色终端主题 ("Solarized Dark")

```
background                  = "#002B36"
foreground                  = "#839496"
cursor                      = "#93A1A1"
cursor_accent               = "#002B36"
selection_background        = "#073642"
selection_foreground        = "#93A1A1"
selection_inactive_background = "#073642"

black                       = "#073642"
red                         = "#DC322F"
green                       = "#859900"
yellow                      = "#B58900"
blue                        = "#268BD2"
magenta                     = "#D33682"
cyan                        = "#2AA198"
white                       = "#EEE8D5"

bright_black                = "#002B36"
bright_red                  = "#CB4B16"
bright_green                = "#586E75"
bright_yellow               = "#657B83"
bright_blue                 = "#839496"
bright_magenta              = "#6C71C4"
bright_cyan                 = "#93A1A1"
bright_white                = "#FDF6E3"
```

### 3.3 默认亮色终端主题 ("Default Light Modern")

```
background                  = "#FFFFFF"
foreground                  = "#333333"
cursor                      = "#333333"
cursor_accent               = "#FFFFFF"
selection_background        = "#ADD6FF"
selection_foreground        = "#000000"
selection_inactive_background = "#ADD6FF"

black                       = "#000000"
red                         = "#CD3131"
green                       = "#429673"
yellow                      = "#949800"
blue                        = "#0451A5"
magenta                     = "#BC05BC"
cyan                        = "#009966"
white                       = "#A5A5A5"

bright_black                = "#666666"
bright_red                  = "#CD3131"
bright_green                = "#429673"
bright_yellow               = "#949800"
bright_blue                 = "#0451A5"
bright_magenta              = "#BC05BC"
bright_cyan                 = "#009966"
bright_white                = "#A5A5A5"
```

---

## 4. 笔记/Markdown 主题 (NoteTheme)

### 4.1 数据结构 (Rust)

```rust
struct NoteTheme {
    theme_name: String,
    originate: String,

    // === 代码块 ===
    note_code_background_color: String,   // 代码块背景
    note_code_border_color: String,       // 代码块边框
    note_code_border_radius: u32,         // 代码块圆角(px)
    note_code_style: String,              // 代码块风格

    // === 行内代码 ===
    note_marker_background_color: String, // 行内代码背景
    note_marker_text_color: String,       // 行内代码文本

    // === 链接 ===
    note_link_color: String,              // 链接颜色
    note_link_underline: String,          // 链接下划线样式

    // === 引用块 ===
    note_quote_border_color: String,      // 引用块左边框色
    note_quote_background_color: String,  // 引用块背景
    note_quote_text_color: String,        // 引用块文本
    note_quote_border_width: u32,         // 引用块左边框宽度(px)

    // === 表格 ===
    note_table_bg_color: String,          // 表格背景
    note_table_border_color: String,      // 表格边框
    note_table_header_bg_color: String,   // 表头背景
    note_table_even_row_bg_color: String, // 偶数行背景
    note_table_border_radius: u32,        // 表格圆角(px)

    // === 标题 ===
    note_h1_color: String,
    note_h2_color: String,
    note_h3_color: String,
    note_h4_color: String,               // H4-H6 共用
}
```

### 4.2 默认暗色笔记主题

```
note_code_background_color    = "#00202B"
note_code_border_color        = "#1A7778"
note_code_border_radius       = 4
note_code_style               = ""

note_marker_background_color  = "#00202B"
note_marker_text_color        = "#FFFFFF"

note_link_color               = "#338CFF"
note_link_underline           = "underline"

note_quote_border_color       = "#105C5D"
note_quote_background_color   = "#023848"
note_quote_text_color         = "#CCCCCC"
note_quote_border_width       = 3

note_table_bg_color           = "#002B36"
note_table_border_color       = "#073642"
note_table_header_bg_color    = "#053747"
note_table_even_row_bg_color  = "#00303F"
note_table_border_radius      = 4

note_h1_color                 = "#FFFFFF"
note_h2_color                 = "#F5F5F5"
note_h3_color                 = "#E0E0E0"
note_h4_color                 = "#CCCCCC"
```

### 4.3 默认亮色笔记主题

```
note_code_background_color    = "#F5F5F5"
note_code_border_color        = "#E0E0E0"
note_code_border_radius       = 4
note_code_style               = ""

note_marker_background_color  = "#EEEEEE"
note_marker_text_color        = "#333333"

note_link_color               = "#1677FF"
note_link_underline           = "underline"

note_quote_border_color       = "#E0E0E0"
note_quote_background_color   = "#F5F5F5"
note_quote_text_color         = "#666666"
note_quote_border_width       = 3

note_table_bg_color           = "#FFFFFF"
note_table_border_color       = "#E0E0E0"
note_table_header_bg_color    = "#EBEBEB"
note_table_even_row_bg_color  = "#F9F9F9"
note_table_border_radius      = 4

note_h1_color                 = "#000000"
note_h2_color                 = "#212121"
note_h3_color                 = "#424242"
note_h4_color                 = "#616161"
```

---

## 5. 扩展主题色 (Extra Theme)

这些颜色不随主题切换而单独管理，而是根据当前 light/dark 模式由前端硬编码选择。

### 5.1 亮色扩展色

```
tab_icon_color           = "#000000"
active_color             = "#007ACC"
search_icon_color        = "#7C868F"
term_connected_color     = "#7EADE2"
edit_disabled_color      = "#C2C2C2"
tab_active_text_color    = "#3599FF"         // 终端 Tab 选中页签

ftp_progress_color       = "#34AB26"
ftp_table_progress_text_color  = "#FFFFFF"
ftp_table_progress_rail_color  = "#D9D9D9"
ftp_progress_border_color     = "#C9C9C9"

info_title_btn_color          = "#007ACB"
info_title_btn_border_color   = "#007ACB"
info_title_btn_hover_bg_color = "#007ACB"

note_tab_header_border    = "#D9D9D9"
note_toolbar_header_bg    = "#F5F5F5"
note_search_num_bg_color  = "#D8D8D8"
outline_hover_color       = "#4285F4"

chat_item_bg_active       = "#DFE0E3"
chat_item_text_active     = "#333333"
chat_item_sub_text_active = "#666666"
chat_code_wrap_bg         = "#EEEEEE"
chat_code_wrap_border     = "#7EA9C9"
chat_code_block_header_copy_bg = "#DEDEDE"

org_tag_bg_color          = "#DEEEFF"
org_tag_text_color        = "#456DCE"

table_th_bg               = "#EBEBEB"
table_tdBg                = "#F9F9F9"
progress_free_bg          = "#EAEAEA"
expand_table_bg           = "#FFFFFF"
table_hover_color         = "#EEEEEE"
```

### 5.2 暗色扩展色

```
tab_icon_color           = "#CCCCCC"
active_color             = "#FFFFFF"
search_icon_color        = "#CCCCCB"
term_connected_color     = "#12A2C5"
edit_disabled_color      = "#FFFFFF61"
tab_active_text_color    = "#FFFFFF"         // 终端 Tab 选中页签

ftp_progress_color       = "#005A6F"
ftp_table_progress_text_color  = "#CCCCCC"
ftp_table_progress_rail_color  = "#00404E"
ftp_progress_border_color     = "#1A7778"

info_title_btn_color          = "#CCCCCC"
info_title_btn_border_color   = "#005A6F"
info_title_btn_hover_bg_color = "transparent"

note_tab_header_border    = "#105C5D"
note_toolbar_header_bg    = "#023848"
note_search_num_bg_color  = "#015367"
outline_hover_color       = "#FFFFFF"

chat_item_bg_active       = "#005A6F"
chat_item_text_active     = "#FFFFFF"
chat_item_sub_text_active = "#CCCCCC"
chat_code_wrap_bg         = "#00202B"
chat_code_wrap_border     = "#1A7778"
chat_code_block_header_copy_bg = "#0C495E"

org_tag_bg_color          = "#458CD033"
org_tag_text_color        = "#6793FF"

table_th_bg               = "#053747"
table_tdBg                = "#00303F"
progress_free_bg          = "#FFFFFF"
expand_table_bg           = "#002733"
table_hover_color         = "#033C4F"
```

---

## 6. 全局 UI 配置常量

这些非颜色配置影响 UI 尺寸和排版，与主题系统配合使用：

```
default_font_family   = "Microsoft YaHei Mono, Microsoft YaHei"
default_font_size     = 13          // 9~22 范围
default_font_bold     = "normal"    // "normal" | "bold"
title_bar_height      = 40
left_nav_font_size    = 13
left_nav_width        = 250
left_host_width       = 300
tabs_max_width        = 180
menu_max_width        = 250
menu_border_radius    = 6
row_height_ratio      = 2
scrollbar_width       = 8
border_radius         = 8           // 组件圆角
border_radius_small   = 4
```

---

## 7. Body 背景色

亮色和暗色模式下 HTML body 的背景色（在主题未加载完成时显示）：

```
light: body_color = "#FFFFFF"
dark:  body_color = "#2E2E35"
```

---

## 8. 终端 Tab 类型颜色

终端 Tab 支持多种颜色标识，用于区分不同类型的连接：

```rust
struct TabTypeConfig {
    name_en: String,
    name_cn: String,
    color: String,              // 未激活文本
    active_color: String,       // 激活文本
    bg_color: String,           // 未激活背景
    active_bg_color: String,    // 激活背景
    border_color: String,       // 未激活边框
    active_border_color: String, // 激活边框
}
```

Tab 颜色跟随系统主题中 `terminal_tab_types` 字段配置，默认使用主题主色调。

---

## 9. Windows Terminal 主题兼容

支持导入 Windows Terminal 的配色方案（`schemes` 数组），字段映射：

| Windows Terminal | WhaleTerminal |
|------------------|---------------|
| `background` | `background` |
| `foreground` | `foreground` |
| `cursorColor` | `cursor` |
| `selectionBackground` | `selection_background` |
| `black` ~ `white` | `black` ~ `white` |
| `brightBlack` ~ `brightWhite` | `brightBlack` ~ `brightWhite` |

---

## 10. 主题持久化与同步

### 10.1 存储结构 (JSON)

```json
{
  "themeLight": { ... },
  "themeDark": { ... },
  "xtermThemeLight": { ... },
  "xtermThemeDark": { ... },
  "noteThemeLight": { ... },
  "noteThemeDark": { ... },
  "defaultThemeLights": [ ... ],
  "defaultThemeDarks": [ ... ],
  "customThemeLights": [ ... ],
  "customThemeDarks": [ ... ],
  "defaultXtermThemeLights": [ ... ],
  "defaultXtermThemeDarks": [ ... ],
  "customXtermThemeLights": [ ... ],
  "customXtermThemeDarks": [ ... ],
  "defaultNoteThemeLights": [ ... ],
  "defaultNoteThemeDarks": [ ... ],
  "customNoteThemeLights": [ ... ],
  "customNoteThemeDarks": [ ... ]
}
```

### 10.2 主题操作

| 操作 | 说明 |
|------|------|
| 选择主题 | 从预设/自定义列表选择 |
| 创建自定义主题 | 基于现有主题复制并修改 |
| 恢复单个色块 | 从 `originate` 来源主题恢复单个颜色 |
| 删除自定义主题 | 删除用户自建主题 |
| 重置默认 | 恢复所有预设主题到出厂值 |

---

## 11. Rust 实现建议

### 11.1 主题 crate 结构

```
src/theme/
├── mod.rs              // 主题管理器
├── system.rs           // SystemTheme 定义 + 默认值
├── terminal.rs         // TerminalTheme 定义 + 默认值
├── note.rs             // NoteTheme 定义 + 默认值
├── extra.rs            // ExtraTheme 扩展色
├── presets/
│   ├── dark.rs         // 暗色预设主题集合
│   └── light.rs        // 亮色预设主题集合
└── storage.rs          // JSON 持久化
```

### 11.2 主题管理器接口

```rust
trait ThemeManager {
    fn current_mode(&self) -> ThemeMode;
    fn set_mode(&mut self, mode: ThemeMode);
    fn system_theme(&self) -> &SystemTheme;
    fn terminal_theme(&self) -> &TerminalTheme;
    fn note_theme(&self) -> &NoteTheme;
    fn extra_colors(&self) -> &ExtraTheme;
    fn on_mode_changed(&self);  // 切换时通知所有组件
}
```

### 11.3 关键注意事项

1. **颜色传递**: 所有颜色使用 `&str` 或 `Color` 类型，支持 hex 格式（`#RRGGBB`）
2. **透明度**: 部分扩展色使用 `#RRGGBBAA` 格式（如 `#458CD033`），需要支持 RGBA 解析
3. **实时切换**: 亮/暗切换时需要通知所有 UI 组件重新读取颜色
4. **CSS 变量映射**: 如果使用 WebView 渲染部分 UI，需将主题色映射为 CSS 变量
5. **原生渲染**: GPU 渲染的终端和笔记编辑器直接从 Theme struct 读取颜色，不经过 CSS
