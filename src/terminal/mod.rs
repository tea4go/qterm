pub mod cell;
pub mod grid;
pub mod parser;
pub mod renderer;

use cell::{CellAttrs, TermColor};
use grid::Grid;

/// 光标位置和可见性
pub struct Cursor {
    pub row: usize,       // 光标行号
    pub col: usize,       // 光标列号
    pub visible: bool,    // 光标是否可见
}

/// 文本选择范围
pub struct Selection {
    pub start_row: usize,  // 选择起始行
    pub start_col: usize,  // 选择起始列
    pub end_row: usize,    // 选择结束行
    pub end_col: usize,    // 选择结束列
}

/// 终端仿真器核心结构体
/// 管理 Grid（字符网格）、光标、颜色属性、滚动区域、选择等
pub struct Terminal {
    pub grid: Grid,                          // 字符网格（含回滚缓冲区）
    pub cursor: Cursor,                      // 当前光标位置
    pub title: String,                       // 终端标题（由 OSC 设置）
    pub saved_cursor: Option<(usize, usize)>, // 保存的光标位置（ESC 7/8）
    pub alt_screen: bool,                    // 是否处于备用屏幕模式
    alt_grid: Option<Grid>,                  // 备用屏幕网格
    pub current_attrs: CellAttrs,            // 当前字符属性（粗体、斜体等）
    pub current_fg: TermColor,               // 当前前景色
    pub current_bg: TermColor,               // 当前背景色
    pub scroll_top: usize,                   // 滚动区域顶部行号
    pub scroll_bottom: usize,                // 滚动区域底部行号
    pub pending_replies: Vec<Vec<u8>>,       // 待发送的 ANSI 响应
    vte_parser: vte::Parser,                 // VTE 解析器
    pub selection: Option<Selection>,        // 当前文本选择
}

impl Terminal {
    /// 创建终端实例
    /// 初始化网格、光标、属性等
    pub fn new(rows: usize, cols: usize, scrollback: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols, scrollback),
            cursor: Cursor { row: 0, col: 0, visible: true },
            title: String::new(),
            saved_cursor: None,
            alt_screen: false,
            alt_grid: None,
            current_attrs: CellAttrs::default(),
            current_fg: TermColor::Default,
            current_bg: TermColor::Default,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            pending_replies: Vec::new(),
            vte_parser: vte::Parser::new(),
            selection: None,
        }
    }

    /// 向终端输入原始字节流
    /// 通过 VTE 解析器处理 ANSI 转义序列，更新网格和光标
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::replace(&mut self.vte_parser, vte::Parser::new());
        for &byte in bytes {
            let mut performer = parser::Performer { terminal: self };
            parser.advance(&mut performer, byte);
        }
        self.vte_parser = parser;
    }

    /// 获取终端行数
    pub fn rows(&self) -> usize {
        self.grid.rows
    }

    /// 获取终端列数
    pub fn cols(&self) -> usize {
        self.grid.cols
    }

    /// 调整终端大小
    /// 重新设置网格尺寸，更新滚动区域和光标位置
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        self.grid.resize(new_rows, new_cols);
        self.scroll_top = 0;
        self.scroll_bottom = new_rows.saturating_sub(1);
        if self.cursor.row >= new_rows {
            self.cursor.row = new_rows.saturating_sub(1);
        }
        if self.cursor.col >= new_cols {
            self.cursor.col = new_cols.saturating_sub(1);
        }
    }

    /// 在滚动区域内向上滚动一行
    pub fn scroll_up_in_region(&mut self) {
        if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
            self.grid.scroll_up();
        } else {
            self.grid.delete_lines(self.scroll_top, 1);
        }
    }

    /// 在滚动区域内向下滚动一行
    pub fn scroll_down_in_region(&mut self) {
        if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
            self.grid.scroll_down();
        } else {
            self.grid.insert_lines(self.scroll_top, 1);
        }
    }

    /// 进入备用屏幕模式（全屏应用如 vim）
    pub fn enter_alt_screen(&mut self) {
        if !self.alt_screen {
            self.alt_screen = true;
            let alt = Grid::new(self.grid.rows, self.grid.cols, 0);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt));
        }
    }

    /// 退出备用屏幕模式，恢复主屏幕内容
    pub fn exit_alt_screen(&mut self) {
        if self.alt_screen {
            self.alt_screen = false;
            if let Some(main_grid) = self.alt_grid.take() {
                self.grid = main_grid;
            }
        }
    }

    /// 获取选中的文本内容
    /// 如果选择范围为空则返回 None
    pub fn selected_text(&self) -> Option<String> {
        let (sr, sc, er, ec) = self.normalized_selection()?;
        if sr == er && sc == ec {
            return None;
        }
        Some(self.grid.text_in_range(sr, sc, er, ec))
    }

    /// 获取标准化后的选择范围（确保起止点有序）
    pub fn normalized_selection(&self) -> Option<(usize, usize, usize, usize)> {
        let sel = self.selection.as_ref()?;
        if (sel.start_row, sel.start_col) <= (sel.end_row, sel.end_col) {
            Some((sel.start_row, sel.start_col, sel.end_row, sel.end_col))
        } else {
            Some((sel.end_row, sel.end_col, sel.start_row, sel.start_col))
        }
    }

    /// 在指定位置查找单词范围（双击选词）
    /// 返回单词的起止行和列
    pub fn word_at(&self, row: usize, col: usize) -> Option<(usize, usize, usize, usize)> {
        if row >= self.rows() || col >= self.cols() {
            return None;
        }
        let row_cells = self.grid.row(row);
        let ch = row_cells[col].ch;
        if ch == ' ' {
            return None;
        }
        // 判断是否为单词字符（非空格、非控制字符）
        let is_word_char = |c: char| c != ' ' && !c.is_control();
        if !is_word_char(ch) {
            return Some((row, col, row, col));
        }
        // 向左扩展单词起始位置
        let mut start = col;
        while start > 0 && is_word_char(row_cells[start - 1].ch) {
            start -= 1;
        }
        // 向右扩展单词结束位置
        let mut end = col;
        while end < self.cols() - 1 && is_word_char(row_cells[end + 1].ch) {
            end += 1;
        }
        Some((row, start, row, end))
    }

    /// 获取指定行的文本范围（三击选行）
    /// 返回从行首到最后一个非空字符的范围
    pub fn line_range(&self, row: usize) -> Option<(usize, usize, usize, usize)> {
        if row >= self.rows() {
            return None;
        }
        let mut end = self.cols() - 1;
        let row_cells = self.grid.row(row);
        // 找到最后一个非空字符
        while end > 0 && row_cells[end].ch == ' ' {
            end -= 1;
        }
        Some((row, 0, row, end))
    }
}