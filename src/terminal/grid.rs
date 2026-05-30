use std::collections::VecDeque;

use super::cell::Cell;

pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    cells: Vec<Vec<Cell>>,
    scrollback: VecDeque<Vec<Cell>>,
    max_scrollback: usize,
    pub scroll_offset: usize,
}

impl Grid {
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

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row][col]
    }

    pub fn row(&self, row: usize) -> &[Cell] {
        &self.cells[row]
    }

    pub fn scroll_up(&mut self) {
        let top_row = self.cells.remove(0);
        self.scrollback.push_back(top_row);
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.pop_front();
        }
        self.cells.push(vec![Cell::default(); self.cols]);
    }

    pub fn scroll_down(&mut self) {
        self.cells.pop();
        self.cells.insert(0, vec![Cell::default(); self.cols]);
    }

    pub fn clear_row(&mut self, row: usize) {
        for cell in &mut self.cells[row] {
            *cell = Cell::default();
        }
    }

    pub fn clear_row_from(&mut self, row: usize, col: usize) {
        for c in col..self.cols {
            self.cells[row][c] = Cell::default();
        }
    }

    pub fn clear_row_to(&mut self, row: usize, col: usize) {
        for c in 0..=col.min(self.cols - 1) {
            self.cells[row][c] = Cell::default();
        }
    }

    pub fn insert_lines(&mut self, row: usize, count: usize) {
        for _ in 0..count {
            if row < self.rows {
                self.cells.pop();
                self.cells.insert(row, vec![Cell::default(); self.cols]);
            }
        }
    }

    pub fn delete_lines(&mut self, row: usize, count: usize) {
        for _ in 0..count {
            if row < self.cells.len() {
                self.cells.remove(row);
                self.cells.push(vec![Cell::default(); self.cols]);
            }
        }
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        self.rows = new_rows;
        self.cols = new_cols;
        self.cells.resize(new_rows, vec![Cell::default(); new_cols]);
        for row in &mut self.cells {
            row.resize(new_cols, Cell::default());
        }
        self.scroll_offset = 0;
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn scrollback_row(&self, idx: usize) -> Option<&[Cell]> {
        self.scrollback.get(idx).map(|r| r.as_slice())
    }

    pub fn text_in_range(&self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) -> String {
        let mut result = String::new();
        for row in start_row..=end_row.min(self.rows - 1) {
            let row_cells = &self.cells[row];
            let col_start = if row == start_row { start_col.min(self.cols) } else { 0 };
            let col_end = if row == end_row { end_col.min(self.cols - 1) } else { self.cols - 1 };
            let mut line_text = String::new();
            for col in col_start..=col_end {
                line_text.push(row_cells[col].ch);
            }
            let trimmed = line_text.trim_end();
            result.push_str(trimmed);
            if row < end_row.min(self.rows - 1) {
                result.push('\n');
            }
        }
        result
    }
}
