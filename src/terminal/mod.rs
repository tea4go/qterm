pub mod cell;
pub mod grid;
pub mod parser;
pub mod renderer;

use cell::{CellAttrs, TermColor};
use grid::Grid;

pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
}

pub struct Terminal {
    pub grid: Grid,
    pub cursor: Cursor,
    pub title: String,
    pub saved_cursor: Option<(usize, usize)>,
    pub alt_screen: bool,
    alt_grid: Option<Grid>,
    pub current_attrs: CellAttrs,
    pub current_fg: TermColor,
    pub current_bg: TermColor,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub pending_replies: Vec<Vec<u8>>,
    vte_parser: vte::Parser,
}

impl Terminal {
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
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::replace(&mut self.vte_parser, vte::Parser::new());
        for &byte in bytes {
            let mut performer = parser::Performer { terminal: self };
            parser.advance(&mut performer, byte);
        }
        self.vte_parser = parser;
    }

    pub fn rows(&self) -> usize {
        self.grid.rows
    }

    pub fn cols(&self) -> usize {
        self.grid.cols
    }

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

    pub fn scroll_up_in_region(&mut self) {
        if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
            self.grid.scroll_up();
        } else {
            self.grid.delete_lines(self.scroll_top, 1);
        }
    }

    pub fn scroll_down_in_region(&mut self) {
        if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
            self.grid.scroll_down();
        } else {
            self.grid.insert_lines(self.scroll_top, 1);
        }
    }

    pub fn enter_alt_screen(&mut self) {
        if !self.alt_screen {
            self.alt_screen = true;
            let alt = Grid::new(self.grid.rows, self.grid.cols, 0);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt));
        }
    }

    pub fn exit_alt_screen(&mut self) {
        if self.alt_screen {
            self.alt_screen = false;
            if let Some(main_grid) = self.alt_grid.take() {
                self.grid = main_grid;
            }
        }
    }
}
