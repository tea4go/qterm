use eframe::egui;

use super::parse_color;

pub struct ExtraTheme {
    // Tab
    pub tab_icon_color: egui::Color32,
    pub tab_active_text_color: egui::Color32,
    pub active_color: egui::Color32,

    // Terminal
    pub term_connected_color: egui::Color32,

    // SFTP progress
    pub ftp_progress_color: egui::Color32,
    pub ftp_progress_rail_color: egui::Color32,
    pub ftp_progress_text_color: egui::Color32,
    pub ftp_progress_border_color: egui::Color32,

    // Table
    pub table_th_bg: egui::Color32,
    pub table_td_bg: egui::Color32,
    pub table_hover_color: egui::Color32,
}

impl ExtraTheme {
    pub fn dark() -> Self {
        Self {
            tab_icon_color: parse_color("#CCCCCC"),
            tab_active_text_color: parse_color("#FFFFFF"),
            active_color: parse_color("#FFFFFF"),

            term_connected_color: parse_color("#12A2C5"),

            ftp_progress_color: parse_color("#005A6F"),
            ftp_progress_rail_color: parse_color("#00404E"),
            ftp_progress_text_color: parse_color("#CCCCCC"),
            ftp_progress_border_color: parse_color("#1A7778"),

            table_th_bg: parse_color("#053747"),
            table_td_bg: parse_color("#00303F"),
            table_hover_color: parse_color("#033C4F"),
        }
    }

    pub fn light() -> Self {
        Self {
            tab_icon_color: parse_color("#000000"),
            tab_active_text_color: parse_color("#3599FF"),
            active_color: parse_color("#007ACC"),

            term_connected_color: parse_color("#7EADE2"),

            ftp_progress_color: parse_color("#34AB26"),
            ftp_progress_rail_color: parse_color("#D9D9D9"),
            ftp_progress_text_color: parse_color("#FFFFFF"),
            ftp_progress_border_color: parse_color("#C9C9C9"),

            table_th_bg: parse_color("#EBEBEB"),
            table_td_bg: parse_color("#F9F9F9"),
            table_hover_color: parse_color("#EEEEEE"),
        }
    }
}
