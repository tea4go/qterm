use super::cell::{CellAttrs, TermColor};
use super::Terminal;

/// VTE 解析器的执行器
/// 将解析后的 ANSI 序列应用到终端状态（光标移动、颜色设置等）
pub struct Performer<'a> {
    pub terminal: &'a mut Terminal,
}

impl<'a> vte::Perform for Performer<'a> {
    /// 处理可打印字符：在光标位置写入字符，光标右移
    fn print(&mut self, c: char) {
        let t = &mut *self.terminal;

        if t.cursor.col >= t.grid.cols {
            t.cursor.col = 0;
            t.cursor.row += 1;
            if t.cursor.row > t.scroll_bottom {
                t.cursor.row = t.scroll_bottom;
                t.scroll_up_in_region();
            }
        }

        let row = t.cursor.row;
        let col = t.cursor.col;
        let cell = t.grid.cell_mut(row, col);
        cell.ch = c;
        cell.fg = t.current_fg;
        cell.bg = t.current_bg;
        cell.attrs = t.current_attrs;
        t.cursor.col += 1;
    }

    /// 处理控制字符执行（退格、制表、换行、回车等）
    fn execute(&mut self, byte: u8) {
        let t = &mut *self.terminal;
        match byte {
            0x08 => {
                if t.cursor.col > 0 {
                    t.cursor.col -= 1;
                }
            }
            0x09 => {
                t.cursor.col = ((t.cursor.col / 8) + 1) * 8;
                if t.cursor.col >= t.grid.cols {
                    t.cursor.col = t.grid.cols - 1;
                }
            }
            0x0A | 0x0B | 0x0C => {
                t.cursor.row += 1;
                if t.cursor.row > t.scroll_bottom {
                    t.cursor.row = t.scroll_bottom;
                    t.scroll_up_in_region();
                }
            }
            0x0D => {
                t.cursor.col = 0;
            }
            _ => {}
        }
    }

    /// 处理 CSI（控制序列引入器）序列
    /// 支持光标移动、清屏、滚动、颜色设置等 ANSI 操作
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let t = &mut *self.terminal;
        let ps: Vec<u16> = params.iter().map(|p| p[0]).collect();
        let p1 = ps.first().copied().unwrap_or(0) as usize;
        let p2 = ps.get(1).copied().unwrap_or(0) as usize;
        let is_private = intermediates.first() == Some(&b'?');

        match action {
            'A' => t.cursor.row = t.cursor.row.saturating_sub(p1.max(1)),
            'B' => t.cursor.row = (t.cursor.row + p1.max(1)).min(t.grid.rows - 1),
            'C' => t.cursor.col = (t.cursor.col + p1.max(1)).min(t.grid.cols - 1),
            'D' => t.cursor.col = t.cursor.col.saturating_sub(p1.max(1)),
            'H' | 'f' => {
                t.cursor.row = (p1.max(1) - 1).min(t.grid.rows - 1);
                t.cursor.col = (p2.max(1) - 1).min(t.grid.cols - 1);
            }
            'J' => match p1 {
                0 => {
                    t.grid.clear_row_from(t.cursor.row, t.cursor.col);
                    for r in (t.cursor.row + 1)..t.grid.rows {
                        t.grid.clear_row(r);
                    }
                }
                1 => {
                    for r in 0..t.cursor.row {
                        t.grid.clear_row(r);
                    }
                    t.grid.clear_row_to(t.cursor.row, t.cursor.col);
                }
                2 | 3 => {
                    for r in 0..t.grid.rows {
                        t.grid.clear_row(r);
                    }
                }
                _ => {}
            },
            'K' => match p1 {
                0 => t.grid.clear_row_from(t.cursor.row, t.cursor.col),
                1 => t.grid.clear_row_to(t.cursor.row, t.cursor.col),
                2 => t.grid.clear_row(t.cursor.row),
                _ => {}
            },
            'L' => t.grid.insert_lines(t.cursor.row, p1.max(1)),
            'M' => t.grid.delete_lines(t.cursor.row, p1.max(1)),
            'S' => {
                for _ in 0..p1.max(1) {
                    t.scroll_up_in_region();
                }
            }
            'T' => {
                for _ in 0..p1.max(1) {
                    t.scroll_down_in_region();
                }
            }
            'r' => {
                let top = p1.max(1) - 1;
                let bottom = if p2 == 0 { t.grid.rows - 1 } else { p2 - 1 };
                t.scroll_top = top.min(t.grid.rows - 1);
                t.scroll_bottom = bottom.min(t.grid.rows - 1);
                t.cursor.row = 0;
                t.cursor.col = 0;
            }
            'h' | 'l' => {
                let enable = action == 'h';
                if is_private {
                    for &p in &ps {
                        match p {
                            25 => t.cursor.visible = enable,
                            1049 => {
                                if enable {
                                    t.enter_alt_screen();
                                } else {
                                    t.exit_alt_screen();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            'm' => {
                let _ = t;
                self.handle_sgr(&ps);
            }
            'd' => t.cursor.row = (p1.max(1) - 1).min(t.grid.rows - 1),
            'G' => t.cursor.col = (p1.max(1) - 1).min(t.grid.cols - 1),
            'n' => {
                if p1 == 6 {
                    // DSR - Device Status Report: report cursor position
                    let reply = format!("\x1b[{};{}R", t.cursor.row + 1, t.cursor.col + 1);
                    t.pending_replies.push(reply.into_bytes());
                }
            }
            'c' => {
                if is_private || p1 == 0 {
                    // DA1 - Device Attributes: report as VT220
                    let reply = b"\x1b[?62;22c".to_vec();
                    t.pending_replies.push(reply);
                }
            }
            _ => {}
        }
    }

    /// 处理 OSC（操作系统命令）序列
    /// 主要用于设置终端标题（OSC 0/1/2）
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        if let Ok(cmd) = std::str::from_utf8(params[0]) {
            match cmd {
                "0" | "1" | "2" => {
                    if params.len() > 1 {
                        if let Ok(title) = std::str::from_utf8(params[1]) {
                            self.terminal.title = title.to_string();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 处理 ESC（转义）序列
    /// 支持保存/恢复光标、索引/反向索引、全复位等
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        let t = &mut *self.terminal;
        match byte {
            b'7' => t.saved_cursor = Some((t.cursor.row, t.cursor.col)),
            b'8' => {
                if let Some((row, col)) = t.saved_cursor {
                    t.cursor.row = row;
                    t.cursor.col = col;
                }
            }
            b'D' => {
                if t.cursor.row == t.scroll_bottom {
                    t.scroll_up_in_region();
                } else if t.cursor.row < t.grid.rows - 1 {
                    t.cursor.row += 1;
                }
            }
            b'M' => {
                if t.cursor.row == t.scroll_top {
                    t.scroll_down_in_region();
                } else if t.cursor.row > 0 {
                    t.cursor.row -= 1;
                }
            }
            b'c' => {
                let rows = t.grid.rows;
                let cols = t.grid.cols;
                *t = Terminal::new(rows, cols, 1000);
            }
            _ => {}
        }
    }

    fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
}

impl<'a> Performer<'a> {
    /// 处理 SGR（选择图形再现）序列
    /// 设置字符属性（粗体、斜体等）和颜色（前景/背景）
    fn handle_sgr(&mut self, params: &[u16]) {
        let t = &mut *self.terminal;
        if params.is_empty() {
            t.current_attrs = CellAttrs::default();
            t.current_fg = TermColor::Default;
            t.current_bg = TermColor::Default;
            return;
        }
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => {
                    t.current_attrs = CellAttrs::default();
                    t.current_fg = TermColor::Default;
                    t.current_bg = TermColor::Default;
                }
                1 => t.current_attrs.bold = true,
                3 => t.current_attrs.italic = true,
                4 => t.current_attrs.underline = true,
                7 => t.current_attrs.inverse = true,
                9 => t.current_attrs.strikethrough = true,
                22 => t.current_attrs.bold = false,
                23 => t.current_attrs.italic = false,
                24 => t.current_attrs.underline = false,
                27 => t.current_attrs.inverse = false,
                29 => t.current_attrs.strikethrough = false,
                30..=37 => t.current_fg = TermColor::Indexed((params[i] - 30) as u8),
                38 => {
                    i += 1;
                    if i < params.len() && params[i] == 5 {
                        i += 1;
                        if i < params.len() {
                            t.current_fg = TermColor::Indexed(params[i] as u8);
                        }
                    } else if i < params.len() && params[i] == 2 {
                        if i + 3 < params.len() {
                            t.current_fg = TermColor::Rgb(
                                params[i + 1] as u8,
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                            );
                            i += 3;
                        }
                    }
                }
                39 => t.current_fg = TermColor::Default,
                40..=47 => t.current_bg = TermColor::Indexed((params[i] - 40) as u8),
                48 => {
                    i += 1;
                    if i < params.len() && params[i] == 5 {
                        i += 1;
                        if i < params.len() {
                            t.current_bg = TermColor::Indexed(params[i] as u8);
                        }
                    } else if i < params.len() && params[i] == 2 {
                        if i + 3 < params.len() {
                            t.current_bg = TermColor::Rgb(
                                params[i + 1] as u8,
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                            );
                            i += 3;
                        }
                    }
                }
                49 => t.current_bg = TermColor::Default,
                90..=97 => t.current_fg = TermColor::Indexed((params[i] - 90 + 8) as u8),
                100..=107 => t.current_bg = TermColor::Indexed((params[i] - 100 + 8) as u8),
                _ => {}
            }
            i += 1;
        }
    }
}