use egui::{Color32, FontId, Pos2, Rect, Ui, Vec2};

use super::Terminal;
use super::cell::TermColor;
use crate::theme::terminal::TerminalTheme;

/// 终端尺寸信息
/// 计算可用空间内能容纳的行数、列数及单元格大小
pub struct TerminalSize {
    pub rows: usize,           // 可容纳的终端行数
    pub cols: usize,           // 可容纳的终端列数
    pub cell_width: f32,       // 单个单元格宽度
    pub cell_height: f32,      // 单个单元格高度
}

/// 终端渲染结果
/// 返回鼠标响应对象和渲染参数（用于后续交互处理）
pub struct RenderResult {
    pub response: egui::Response,  // 鼠标交互响应
    pub cell_width: f32,           // 单元格宽度
    pub cell_height: f32,          // 单元格高度
    pub origin: Pos2,             // 终端绘制起点坐标
}

/// 根据可用 UI 空间和字体大小计算终端能容纳的行列数
pub fn calculate_size(ui: &Ui, font_size: f32) -> TerminalSize {
    let font_id = FontId::monospace(font_size);
    // 使用 'M' 字符宽度作为等宽字体的单元格宽度基准
    let cell_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let cell_height = font_size * 1.4;  // 行高为字体大小的 1.4 倍
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

/// 渲染终端内容到 egui UI
/// 绘制背景、字符、选区高亮和光标
pub fn render(ui: &mut Ui, terminal: &Terminal, theme: &TerminalTheme) -> RenderResult {
    let font_id = FontId::monospace(theme.font_size);
    let cell_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let cell_height = theme.font_size * 1.4;

    let available = ui.available_size();
    let render_width = available.x.max(terminal.cols() as f32 * cell_width);
    let render_height = available.y.max(terminal.rows() as f32 * cell_height);

    // 分配绘制区域和交互感知
    let (response, painter) = ui.allocate_painter(
        Vec2::new(render_width, render_height),
        egui::Sense::click_and_drag(),
    );
    let origin = response.rect.min;

    // 绘制背景填充
    painter.rect_filled(response.rect, 0.0, theme.background);

    // 渲染每个单元格
    for row_idx in 0..terminal.rows() {
        let y = origin.y + row_idx as f32 * cell_height;
        let grid_row = terminal.grid.row(row_idx);

        // 绘制非默认背景色单元格的背景
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

        // 按颜色分段绘制文本（优化绘制性能）
        let mut col = 0;
        while col < grid_row.len() {
            let start_col = col;
            let fg = resolve_fg(grid_row[col].fg, grid_row[col].attrs.inverse, theme);
            let mut text = String::new();
            // 连续相同前景色的单元格合并为一次绘制
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

    // 绘制选区高亮
    if let Some((sr, sc, er, ec)) = terminal.normalized_selection() {
        for row in sr..=er.min(terminal.rows() - 1) {
            let col_start = if row == sr { sc } else { 0 };
            let col_end = if row == er { ec.min(terminal.cols() - 1) } else { terminal.cols() - 1 };
            let y = origin.y + row as f32 * cell_height;
            let x_start = origin.x + col_start as f32 * cell_width;
            let x_end = origin.x + (col_end + 1) as f32 * cell_width;
            let rect = Rect::from_min_size(
                Pos2::new(x_start, y),
                Vec2::new(x_end - x_start, cell_height),
            );
            // 绘制选区背景色
            painter.rect_filled(rect, 0.0, theme.selection_bg);
            // 在选区背景上重新绘制文本（使用选区前景色）
            let grid_row = terminal.grid.row(row);
            let mut col = col_start;
            while col <= col_end {
                let start_col = col;
                let fg = theme.selection_fg;
                let mut text = String::new();
                while col <= col_end {
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
    }

    // 绘制光标
    if terminal.cursor.visible && terminal.cursor.row < terminal.rows() {
        let cx = origin.x + terminal.cursor.col as f32 * cell_width;
        let cy = origin.y + terminal.cursor.row as f32 * cell_height;
        let cursor_rect = Rect::from_min_size(
            Pos2::new(cx, cy),
            Vec2::new(cell_width, cell_height),
        );
        // 绘制光标背景色方块
        painter.rect_filled(cursor_rect, 0.0, theme.cursor);
        // 在光标方块上绘制当前字符（使用光标强调色）
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

    RenderResult {
        response,
        cell_width,
        cell_height,
        origin,
    }
}

/// 解析前景色：考虑反色模式时交换前景/背景
fn resolve_fg(color: TermColor, inverse: bool, theme: &TerminalTheme) -> Color32 {
    if inverse {
        color.to_color32(false, theme)
    } else {
        color.to_color32(true, theme)
    }
}

/// 解析背景色：考虑反色模式时交换前景/背景
fn resolve_bg(color: TermColor, inverse: bool, theme: &TerminalTheme) -> Color32 {
    if inverse {
        color.to_color32(true, theme)
    } else {
        color.to_color32(false, theme)
    }
}