use eframe::egui;
use crate::ssh::{SshAuth, SshConfig};

#[derive(PartialEq)]
enum AuthMode {
    Password,
    PrivateKey,
}

pub struct SshDialog {
    pub open: bool,
    host: String,
    port: String,
    username: String,
    password: String,
    key_path: String,
    key_passphrase: String,
    auth_mode: AuthMode,
    pub status: Option<String>,
    pub result: Option<SshConfig>,
}

impl SshDialog {
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

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        egui::Window::new("SSH Connection")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("ssh_form").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.host);
                    ui.end_row();

                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port);
                    ui.end_row();

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                    ui.end_row();
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.auth_mode, AuthMode::Password, "Password");
                    ui.radio_value(&mut self.auth_mode, AuthMode::PrivateKey, "Private Key");
                });

                egui::Grid::new("ssh_auth").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    match self.auth_mode {
                        AuthMode::Password => {
                            ui.label("Password:");
                            ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                            ui.end_row();
                        }
                        AuthMode::PrivateKey => {
                            ui.label("Key file:");
                            ui.text_edit_singleline(&mut self.key_path);
                            ui.end_row();

                            ui.label("Passphrase:");
                            ui.add(egui::TextEdit::singleline(&mut self.key_passphrase).password(true));
                            ui.end_row();
                        }
                    }
                });

                if let Some(status) = &self.status {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), status);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Connect").clicked() {
                        self.try_connect();
                    }
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        self.status = None;
                    }
                });
            });
    }

    fn try_connect(&mut self) {
        let port: u16 = self.port.parse().unwrap_or(22);
        if self.host.is_empty() || self.username.is_empty() {
            self.status = Some("Host and username are required".to_string());
            return;
        }
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
