use std::collections::VecDeque;

use super::cell::Cell;

/// 终端字符网格
/// 管理屏幕上的字符单元格、回滚缓冲区和滚动偏移
pub struct Grid {
    pub rows: usize,                // 屏幕行数
    pub cols: usize,                // 屏幕列数
    cells: Vec<Vec<Cell>>,         // 当前屏幕的单元格矩阵
    scrollback: VecDeque<Vec<Cell>>, // 回滚缓冲区（存储滚出屏幕的历史行）
    max_scrollback: usize,         // 回滚缓冲区最大行数
    pub scroll_offset: usize,      // 当前滚动偏移（0 = 不滚动）
}

impl Grid {
    /// 创建网格实例
    /// 初始化指定行列的单元格矩阵和回滚缓冲区
    pub fn new(rows: usize, cols: usize, max_scrollback: usize) -> Self {
        let cells = vec![vec![Cell::default(); cols]; rows];
        Self {
            rows,
            cols,
            cells,
            scrollback: VecDeque::new(),
            max_scrollback,
            scroll_offset: 0,
        }
    }

    /// 获取指定位置的单元格引用
    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    /// 获取指定位置的单元格可变引用
    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row][col]
    }

    /// 获取指定行的单元格切片引用
    pub fn row(&self, row: usize) -> &[Cell] {
        &self.cells[row]
    }

    /// 向上滚动一行：将顶行移入回滚缓冲区，底部添加新空行
    pub fn scroll_up(&mut self) {
        let top_row = self.cells.remove(0);
        self.scrollback.push_back(top_row);
        // 超过最大回滚行数时移除最旧的行
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.pop_front();
        }
        self.cells.push(vec![Cell::default(); self.cols]);
    }

    /// 向下滚动一行：移除底部行，顶部添加新空行
    pub fn scroll_down(&mut self) {
        self.cells.pop();
        self.cells.insert(0, vec![Cell::default(); self.cols]);
    }

    /// 清除指定行的所有单元格
    pub fn clear_row(&mut self, row: usize) {
        for cell in &mut self.cells[row] {
            *cell = Cell::default();
        }
    }

    /// 清除指定行从指定列到行尾的所有单元格
    pub fn clear_row_from(&mut self, row: usize, col: usize) {
        for c in col..self.cols {
            self.cells[row][c] = Cell::default();
        }
    }

    /// 清除指定行从行首到指定列的所有单元格
    pub fn clear_row_to(&mut self, row: usize, col: usize) {
        for c in 0..=col.min(self.cols - 1) {
            self.cells[row][c] = Cell::default();
        }
    }

    /// 在指定行位置插入空行（向下推动）
    pub fn insert_lines(&mut self, row: usize, count: usize) {
        for _ in 0..count {
            if row < self.rows {
                self.cells.pop();
                self.cells.insert(row, vec![Cell::default(); self.cols]);
            }
        }
    }

    /// 从指定行位置删除行（向上推动，底部补空行）
    pub fn delete_lines(&mut self, row: usize, count: usize) {
        for _ in 0..count {
            if row < self.cells.len() {
                self.cells.remove(row);
                self.cells.push(vec![Cell::default(); self.cols]);
            }
        }
    }

    /// 调整网格大小
    /// 重新设置行列数，调整单元格矩阵和回滚缓冲区
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        self.rows = new_rows;
        self.cols = new_cols;
        self.cells.resize(new_rows, vec![Cell::default(); new_cols]);
        for row in &mut self.cells {
            row.resize(new_cols, Cell::default());
        }
        self.scroll_offset = 0;
    }

    /// 获取回滚缓冲区行数
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// 获取回滚缓冲区中指定索引行的单元格切片
    pub fn scrollback_row(&self, idx: usize) -> Option<&[Cell]> {
        self.scrollback.get(idx).map(|r| r.as_slice())
    }

    /// 获取指定行范围内的文本内容
    /// 用于复制选中文本
    pub fn text_in_range(&self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) -> String {
        let mut result = String::new();
        for row in start_row..=end_row.min(self.rows - 1) {
            let row_cells = &self.cells[row];
            // 首行和末行使用指定的起始/结束列，中间行使用全行
            let col_start = if row == start_row { start_col.min(self.cols) } else { 0 };
            let col_end = if row == end_row { end_col.min(self.cols - 1) } else { self.cols - 1 };
            let mut line_text = String::new();
            for col in col_start..=col_end {
                line_text.push(row_cells[col].ch);
            }
            // 去除行尾空格
            let trimmed = line_text.trim_end();
            result.push_str(trimmed);
            if row < end_row.min(self.rows - 1) {
                result.push('\n');
            }
        }
        result
    }
}