use egui::Color32;

pub struct TermTheme {
    pub font_size: f32,
    pub background: Color32,
    pub foreground: Color32,
    pub cursor_color: Color32,
    pub selection_bg: Color32,
    pub ansi_colors: [Color32; 16],
}

impl TermTheme {
    pub fn dark() -> Self {
        Self {
            font_size: 14.0,
            background: Color32::from_rgb(30, 30, 30),
            foreground: Color32::from_rgb(204, 204, 204),
            cursor_color: Color32::from_rgb(255, 255, 255),
            selection_bg: Color32::from_rgba_premultiplied(70, 130, 180, 100),
            ansi_colors: [
                Color32::from_rgb(0, 0, 0),       // black
                Color32::from_rgb(205, 49, 49),   // red
                Color32::from_rgb(13, 188, 121),  // green
                Color32::from_rgb(229, 229, 16),  // yellow
                Color32::from_rgb(36, 114, 200),  // blue
                Color32::from_rgb(188, 63, 188),  // magenta
                Color32::from_rgb(17, 168, 205),  // cyan
                Color32::from_rgb(204, 204, 204), // white
                Color32::from_rgb(102, 102, 102), // bright black
                Color32::from_rgb(241, 76, 76),   // bright red
                Color32::from_rgb(35, 209, 139),  // bright green
                Color32::from_rgb(245, 245, 67),  // bright yellow
                Color32::from_rgb(59, 142, 234),  // bright blue
                Color32::from_rgb(214, 112, 214), // bright magenta
                Color32::from_rgb(41, 184, 219),  // bright cyan
                Color32::from_rgb(242, 242, 242), // bright white
            ],
        }
    }

    pub fn color_from_index(&self, idx: u8) -> Color32 {
        if idx < 16 {
            self.ansi_colors[idx as usize]
        } else if idx < 232 {
            let i = idx - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let to_val = |c: u8| if c == 0 { 0u8 } else { 55 + 40 * c };
            Color32::from_rgb(to_val(r), to_val(g), to_val(b))
        } else {
            let gray = 8 + 10 * (idx - 232);
            Color32::from_rgb(gray, gray, gray)
        }
    }
}
