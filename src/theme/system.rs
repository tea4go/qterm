use eframe::egui;

use super::parse_color;

pub struct SystemTheme {
    // 应用基础
    pub text_color: egui::Color32,
    pub text_active_color: egui::Color32,
    pub app_bg_color: egui::Color32,
    pub app_divider_color: egui::Color32,
    pub app_split_color: egui::Color32,
    pub border_color: egui::Color32,

    // 头部
    pub app_header_text_color: egui::Color32,

    // 侧边栏
    pub app_sider_bar_bg_color: egui::Color32,
    pub app_side_hover_bg_color: egui::Color32,
    pub app_side_text_active_color: egui::Color32,
    pub app_side_text_color: egui::Color32,

    // 状态栏
    pub app_status_bar_bg_color: egui::Color32,
    pub app_status_bar_text_color: egui::Color32,
    pub app_status_bar_text_hover_color: egui::Color32,

    // 左侧列表
    pub app_left_list_bg_color: egui::Color32,
    pub app_left_list_bg_color_hover: egui::Color32,
    pub app_left_list_bg_color_active: egui::Color32,
    pub app_left_list_text_color_active: egui::Color32,

    // 右侧内容
    pub app_content_term_bg_color: egui::Color32,

    // 弹出层
    pub dialog_bg_color: egui::Color32,
    pub dialog_border_color: egui::Color32,
    pub dialog_divider_color: egui::Color32,
    pub dialog_text_color: egui::Color32,
    pub dialog_text_active_color: egui::Color32,

    // 下拉菜单
    pub drop_down_color: egui::Color32,
    pub drop_down_bg_color: egui::Color32,
    pub drop_down_active_color: egui::Color32,
    pub drop_down_active_bg_color: egui::Color32,

    // 输入框
    pub input_content_bg_color: egui::Color32,
    pub input_content_border_color: egui::Color32,

    // 表格
    pub table_bg_color: egui::Color32,
    pub table_border_color: egui::Color32,
    pub table_header_bg_color: egui::Color32,
    pub table_even_row_bg_color: egui::Color32,
}

impl SystemTheme {
    pub fn dark() -> Self {
        Self {
            text_color: parse_color("#FFFFFF"),
            text_active_color: parse_color("#FFFFFF"),
            app_bg_color: parse_color("#002B36"),
            app_divider_color: parse_color("#073642"),
            app_split_color: parse_color("#073642"),
            border_color: parse_color("#1A7778"),

            app_header_text_color: parse_color("#FFFFFF"),

            app_sider_bar_bg_color: parse_color("#073642"),
            app_side_hover_bg_color: parse_color("#073642"),
            app_side_text_active_color: parse_color("#FFFFFF"),
            app_side_text_color: parse_color("#CCCCCC"),

            app_status_bar_bg_color: parse_color("#002B36"),
            app_status_bar_text_color: parse_color("#CCCCCC"),
            app_status_bar_text_hover_color: parse_color("#FFFFFF"),

            app_left_list_bg_color: parse_color("#073642"),
            app_left_list_bg_color_hover: parse_color("#09495E"),
            app_left_list_bg_color_active: parse_color("#094771"),
            app_left_list_text_color_active: parse_color("#FFFFFF"),

            app_content_term_bg_color: parse_color("#000000"),

            dialog_bg_color: parse_color("#00222B"),
            dialog_border_color: parse_color("#1A7778"),
            dialog_divider_color: parse_color("#073642"),
            dialog_text_color: parse_color("#CCCCCC"),
            dialog_text_active_color: parse_color("#FFFFFF"),

            drop_down_color: parse_color("#CCCCCC"),
            drop_down_bg_color: parse_color("#002B36"),
            drop_down_active_color: parse_color("#FFFFFF"),
            drop_down_active_bg_color: parse_color("#09495E"),

            input_content_bg_color: parse_color("#00222B"),
            input_content_border_color: parse_color("#1A7778"),

            table_bg_color: parse_color("#002B36"),
            table_border_color: parse_color("#073642"),
            table_header_bg_color: parse_color("#073642"),
            table_even_row_bg_color: parse_color("#00222B"),
        }
    }

    pub fn light() -> Self {
        Self {
            text_color: parse_color("#333333"),
            text_active_color: parse_color("#007ACC"),
            app_bg_color: parse_color("#F5F5F5"),
            app_divider_color: parse_color("#E0E0E0"),
            app_split_color: parse_color("#E0E0E0"),
            border_color: parse_color("#CCCCCC"),

            app_header_text_color: parse_color("#333333"),

            app_sider_bar_bg_color: parse_color("#FFFFFF"),
            app_side_hover_bg_color: parse_color("#E3F2FD"),
            app_side_text_active_color: parse_color("#007ACC"),
            app_side_text_color: parse_color("#666666"),

            app_status_bar_bg_color: parse_color("#F5F5F5"),
            app_status_bar_text_color: parse_color("#666666"),
            app_status_bar_text_hover_color: parse_color("#333333"),

            app_left_list_bg_color: parse_color("#FFFFFF"),
            app_left_list_bg_color_hover: parse_color("#F5F5F5"),
            app_left_list_bg_color_active: parse_color("#E3F2FD"),
            app_left_list_text_color_active: parse_color("#007ACC"),

            app_content_term_bg_color: parse_color("#FFFFFF"),

            dialog_bg_color: parse_color("#FFFFFF"),
            dialog_border_color: parse_color("#E0E0E0"),
            dialog_divider_color: parse_color("#E0E0E0"),
            dialog_text_color: parse_color("#333333"),
            dialog_text_active_color: parse_color("#007ACC"),

            drop_down_color: parse_color("#333333"),
            drop_down_bg_color: parse_color("#FFFFFF"),
            drop_down_active_color: parse_color("#007ACC"),
            drop_down_active_bg_color: parse_color("#E3F2FD"),

            input_content_bg_color: parse_color("#FFFFFF"),
            input_content_border_color: parse_color("#CCCCCC"),

            table_bg_color: parse_color("#FFFFFF"),
            table_border_color: parse_color("#E0E0E0"),
            table_header_bg_color: parse_color("#F5F5F5"),
            table_even_row_bg_color: parse_color("#F9F9F9"),
        }
    }

    /// Apply this SystemTheme to the egui global Style/Visuals.
    /// This ensures all built-in egui widgets (buttons, inputs, menus, etc.)
    /// automatically use our theme colors.
    pub fn apply_to_egui(&self, ctx: &egui::Context, is_dark: bool) {
        let mut style = (*ctx.style()).clone();

        // --- Visuals ---
        style.visuals.dark_mode = is_dark;

        // Window / Panel / Popup
        style.visuals.panel_fill = self.app_bg_color;
        style.visuals.window_fill = self.dialog_bg_color;
        style.visuals.window_stroke = egui::Stroke::new(1.0, self.dialog_border_color);
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        style.visuals.extreme_bg_color = self.input_content_bg_color;
        style.visuals.faint_bg_color = if is_dark {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 3)
        } else {
            egui::Color32::from_rgba_premultiplied(0, 0, 0, 3)
        };

        // Popup shadow (egui uses shadow for popups/menus)
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: egui::Vec2::new(2.0, 4.0),
            blur: 12.0,
            spread: 0.0,
            color: if is_dark {
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 96)
            } else {
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 48)
            },
        };

        // Selection
        style.visuals.selection = egui::style::Selection {
            bg_fill: if is_dark {
                parse_color("#094771")
            } else {
                parse_color("#ADD6FF")
            },
            stroke: egui::Stroke::new(1.0, self.text_active_color),
        };

        // Hyperlink
        style.visuals.hyperlink_color = if is_dark {
            parse_color("#268BD2")
        } else {
            parse_color("#0451A5")
        };

        // Error / Warning colors
        style.visuals.error_fg_color = if is_dark {
            parse_color("#DC322F")
        } else {
            parse_color("#CD3131")
        };
        style.visuals.warn_fg_color = if is_dark {
            parse_color("#B58900")
        } else {
            parse_color("#949800")
        };

        // Widgets: noninteractive (backgrounds, labels, general UI)
        style.visuals.widgets.noninteractive.bg_fill = self.app_bg_color;
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.text_color);
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.border_color);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.noninteractive.weak_bg_fill = self.app_bg_color;

        // Widgets: inactive (buttons, unselected tabs)
        style.visuals.widgets.inactive.bg_fill = self.app_sider_bar_bg_color;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.app_side_text_color);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, self.border_color);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.inactive.weak_bg_fill = self.app_sider_bar_bg_color;

        // Widgets: hovered (button hover, tab hover)
        style.visuals.widgets.hovered.bg_fill = self.app_side_hover_bg_color;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, self.text_active_color);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, self.text_active_color);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.hovered.weak_bg_fill = self.app_side_hover_bg_color;

        // Widgets: active (pressed button, active tab)
        style.visuals.widgets.active.bg_fill = self.app_side_hover_bg_color;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, self.text_active_color);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, self.text_active_color);
        style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
        style.visuals.widgets.active.weak_bg_fill = self.app_side_hover_bg_color;

        // Widgets: open (open popup, dropdown)
        style.visuals.widgets.open.bg_fill = self.drop_down_active_bg_color;
        style.visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, self.drop_down_active_color);
        style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, self.border_color);

        // Override text color globally
        style.visuals.override_text_color = Some(self.text_color);

        // Striped table rows
        style.visuals.striped = true;

        // --- Spacing ---
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.indent = 18.0;
        style.spacing.window_margin = egui::Margin::same(12.0);

        // Scrollbar
        style.spacing.scroll.bar_width = 12.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.bar_outer_margin = 0.0;

        // --- Text Styles ---
        let font_size = 13.0;
        let text_styles = [
            (egui::TextStyle::Small, 11.0),
            (egui::TextStyle::Body, font_size),
            (egui::TextStyle::Monospace, font_size),
            (egui::TextStyle::Button, font_size),
            (egui::TextStyle::Heading, font_size + 5.0),
        ];
        for (style_text, size) in text_styles {
            style
                .text_styles
                .entry(style_text)
                .or_insert_with(|| {
                    egui::FontId::proportional(size)
                })
                .size = size;
        }

        ctx.set_style(style);
    }
}
