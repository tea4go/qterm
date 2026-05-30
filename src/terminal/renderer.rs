use egui::{Color32, FontId, Pos2, Rect, Ui, Vec2};

use super::Terminal;
use super::cell::TermColor;
use crate::theme::terminal::TerminalTheme;

pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
    pub cell_width: f32,
    pub cell_height: f32,
}

pub fn calculate_size(ui: &Ui, font_size: f32) -> TerminalSize {
    let font_id = FontId::monospace(font_size);
    let cell_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let cell_height = font_size * 1.4;
    let available = ui.available_size();
    let cols = (available.x / cell_width).floor() as usize;
    let rows = (available.y / cell_height).floor() as usize;
    TerminalSize {
        rows: rows.max(1),
        cols: cols.max(1),
        cell_width,
        cell_height,
    }
}

pub fn render(ui: &mut Ui, terminal: &Terminal, theme: &TerminalTheme) {
    let font_id = FontId::monospace(theme.font_size);
    let cell_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let cell_height = theme.font_size * 1.4;

    let available = ui.available_size();
    let render_width = available.x.max(terminal.cols() as f32 * cell_width);
    let render_height = available.y.max(terminal.rows() as f32 * cell_height);

    let (response, painter) = ui.allocate_painter(
        Vec2::new(render_width, render_height),
        egui::Sense::click_and_drag(),
    );
    let origin = response.rect.min;

    // Draw background
    painter.rect_filled(response.rect, 0.0, theme.background);

    // Render cells
    for row_idx in 0..terminal.rows() {
        let y = origin.y + row_idx as f32 * cell_height;
        let grid_row = terminal.grid.row(row_idx);

        // Draw background colors for non-default cells
        for (col_idx, cell) in grid_row.iter().enumerate() {
            let bg = resolve_bg(cell.bg, cell.attrs.inverse, theme);
            if bg != theme.background {
                let rect = Rect::from_min_size(
                    Pos2::new(origin.x + col_idx as f32 * cell_width, y),
                    Vec2::new(cell_width, cell_height),
                );
                painter.rect_filled(rect, 0.0, bg);
            }
        }

        // Draw text in runs of same color
        let mut col = 0;
        while col < grid_row.len() {
            let start_col = col;
            let fg = resolve_fg(grid_row[col].fg, grid_row[col].attrs.inverse, theme);
            let mut text = String::new();
            while col < grid_row.len() {
                let cell_fg = resolve_fg(grid_row[col].fg, grid_row[col].attrs.inverse, theme);
                if cell_fg != fg {
                    break;
                }
                text.push(grid_row[col].ch);
                col += 1;
            }
            let trimmed = text.trim_end();
            if !trimmed.is_empty() {
                let x = origin.x + start_col as f32 * cell_width;
                painter.text(
                    Pos2::new(x, y),
                    egui::Align2::LEFT_TOP,
                    trimmed,
                    font_id.clone(),
                    fg,
                );
            }
        }
    }

    // Draw cursor
    if terminal.cursor.visible && terminal.cursor.row < terminal.rows() {
        let cx = origin.x + terminal.cursor.col as f32 * cell_width;
        let cy = origin.y + terminal.cursor.row as f32 * cell_height;
        let cursor_rect = Rect::from_min_size(
            Pos2::new(cx, cy),
            Vec2::new(cell_width, cell_height),
        );
        painter.rect_filled(cursor_rect, 0.0, theme.cursor);
        if terminal.cursor.col < terminal.cols() {
            let ch = terminal.grid.row(terminal.cursor.row)[terminal.cursor.col].ch;
            if ch != ' ' {
                painter.text(
                    Pos2::new(cx, cy),
                    egui::Align2::LEFT_TOP,
                    ch.to_string(),
                    font_id.clone(),
                    theme.cursor_accent,
                );
            }
        }
    }
}

fn resolve_fg(color: TermColor, inverse: bool, theme: &TerminalTheme) -> Color32 {
    if inverse {
        color.to_color32(false, theme)
    } else {
        color.to_color32(true, theme)
    }
}

fn resolve_bg(color: TermColor, inverse: bool, theme: &TerminalTheme) -> Color32 {
    if inverse {
        color.to_color32(true, theme)
    } else {
        color.to_color32(false, theme)
    }
}