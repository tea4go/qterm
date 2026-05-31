use eframe::egui;
use crate::ssh::{SshAuth, SshConfig};

/// 认证模式选择
#[derive(PartialEq)]
enum AuthMode {
    Password,    // 密码认证
    PrivateKey,  // 私钥认证
}

/// SSH 连接对话框
/// 弹出窗口用于输入 SSH 连接参数并建立连接
pub struct SshDialog {
    pub open: bool,              // 对话框是否打开
    host: String,                // 主机地址
    port: String,                // 端口号
    username: String,            // 用户名
    password: String,            // 密码
    key_path: String,            // 私钥文件路径
    key_passphrase: String,      // 私钥密码
    auth_mode: AuthMode,         // 当前选择的认证模式
    pub status: Option<String>,  // 状态/错误信息
    pub result: Option<SshConfig>, // 连接配置结果（用于传递给主逻辑）
}

impl SshDialog {
    /// 创建 SSH 连接对话框实例
    pub fn new() -> Self {
        Self {
            open: false,
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            password: String::new(),
            key_path: String::new(),
            key_passphrase: String::new(),
            auth_mode: AuthMode::Password,
            status: None,
            result: None,
        }
    }

    /// 显示 SSH 连接对话框
    /// 当 open 为 true 时弹出模态窗口
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        egui::Window::new("SSH 连接")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 基本连接参数表单
                egui::Grid::new("ssh_form").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label("主机：");
                    ui.text_edit_singleline(&mut self.host);
                    ui.end_row();

                    ui.label("端口：");
                    ui.text_edit_singleline(&mut self.port);
                    ui.end_row();

                    ui.label("用户名：");
                    ui.text_edit_singleline(&mut self.username);
                    ui.end_row();
                });

                ui.separator();
                // 认证模式选择
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.auth_mode, AuthMode::Password, "密码");
                    ui.radio_value(&mut self.auth_mode, AuthMode::PrivateKey, "私钥");
                });

                // 根据认证模式显示不同的输入字段
                egui::Grid::new("ssh_auth").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    match self.auth_mode {
                        AuthMode::Password => {
                            ui.label("密码：");
                            ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                            ui.end_row();
                        }
                        AuthMode::PrivateKey => {
                            ui.label("密钥文件：");
                            ui.text_edit_singleline(&mut self.key_path);
                            ui.end_row();

                            ui.label("密钥密码：");
                            ui.add(egui::TextEdit::singleline(&mut self.key_passphrase).password(true));
                            ui.end_row();
                        }
                    }
                });

                // 显示错误状态
                if let Some(status) = &self.status {
                    ui.colored_label(ui.visuals().error_fg_color, status);
                }

                ui.separator();
                // 连接和取消按钮
                ui.horizontal(|ui| {
                    if ui.button("连接").clicked() {
                        self.try_connect();
                    }
                    if ui.button("取消").clicked() {
                        self.open = false;
                        self.status = None;
                    }
                });
            });
    }

    /// 尝试建立 SSH 连接
    /// 验证必填字段后生成连接配置
    fn try_connect(&mut self) {
        let port: u16 = self.port.parse().unwrap_or(22);
        // 验证必填字段
        if self.host.is_empty() || self.username.is_empty() {
            self.status = Some("主机和用户名为必填项".to_string());
            return;
        }
        // 根据认证模式生成认证配置
        let auth = match self.auth_mode {
            AuthMode::Password => SshAuth::Password(self.password.clone()),
            AuthMode::PrivateKey => SshAuth::PrivateKey {
                path: self.key_path.clone(),
                passphrase: if self.key_passphrase.is_empty() {
                    None
                } else {
                    Some(self.key_passphrase.clone())
                },
            },
        };
        // 生成连接配置并关闭对话框
        self.result = Some(SshConfig {
            host: self.host.clone(),
            port,
            username: self.username.clone(),
            auth,
            timeout_secs: 5,
        });
        self.open = false;
        self.status = None;
    }
}