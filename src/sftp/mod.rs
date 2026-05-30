use std::sync::{Arc, mpsc::{self, Receiver}, atomic::{AtomicBool, Ordering}};
use tokio::runtime::Runtime;
use russh_sftp::client::SftpSession;

use crate::ssh::SharedSshHandle;

pub struct SftpHandle {
    events_rx: Receiver<SftpEvent>,
    cmd_tx: tokio::sync::mpsc::Sender<SftpCommand>,
    alive: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub enum SftpEvent {
    Connected,
    DirListing(Vec<FileEntry>),
    UploadDone(Result<(), String>),
    DownloadDone(Result<(), String>),
    MkdirDone(Result<(), String>),
    DeleteDone(Result<(), String>),
    Error(String),
}

enum SftpCommand {
    ListDir(String),
    Upload { local_path: String, remote_path: String },
    Download { remote_path: String, local_path: String },
    Mkdir(String),
    Delete { path: String, is_dir: bool },
    Disconnect,
}

impl SftpHandle {
    pub fn new(
        ssh_handle: SharedSshHandle,
        rt: &Runtime,
    ) -> Result<Self, crate::ssh::SshError> {
        let (events_tx, events_rx) = mpsc::channel::<SftpEvent>();
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<SftpCommand>(256);
        let alive = Arc::new(AtomicBool::new(true));

        let alive_clone = alive.clone();
        rt.spawn(async move {
            sftp_task(ssh_handle, events_tx, cmd_rx, alive_clone).await;
        });

        Ok(Self {
            events_rx,
            cmd_tx,
            alive,
        })
    }

    pub fn poll(&self) -> Vec<SftpEvent> {
        let mut events = Vec::new();
        while let Ok(e) = self.events_rx.try_recv() {
            events.push(e);
        }
        events
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn list_dir(&self, path: &str) {
        let _ = self.cmd_tx.try_send(SftpCommand::ListDir(path.to_string()));
    }

    pub fn upload(&self, local_path: String, remote_path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Upload { local_path, remote_path });
    }

    pub fn download(&self, remote_path: String, local_path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Download { remote_path, local_path });
    }

    pub fn mkdir(&self, path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Mkdir(path));
    }

    pub fn delete(&self, path: String, is_dir: bool) {
        let _ = self.cmd_tx.try_send(SftpCommand::Delete { path, is_dir });
    }

    pub fn disconnect(&self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.cmd_tx.try_send(SftpCommand::Disconnect);
    }
}

async fn sftp_task(
    ssh_handle: SharedSshHandle,
    events_tx: mpsc::Sender<SftpEvent>,
    mut cmd_rx: tokio::sync::mpsc::Receiver<SftpCommand>,
    alive: Arc<AtomicBool>,
) {
    let sftp = {
        let h = ssh_handle.lock().await;
        match h.channel_open_session().await {
            Ok(channel) => {
                if let Err(e) = channel.request_subsystem(true, "sftp").await {
                    let _ = events_tx.send(SftpEvent::Error(format!("SFTP subsystem request failed: {}", e)));
                    return;
                }
                match SftpSession::new(channel.into_stream()).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = events_tx.send(SftpEvent::Error(format!("SFTP session init failed: {}", e)));
                        return;
                    }
                }
            }
            Err(e) => {
                let _ = events_tx.send(SftpEvent::Error(format!("Failed to open SFTP channel: {}", e)));
                return;
            }
        }
    };

    let _ = events_tx.send(SftpEvent::Connected);

    while alive.load(Ordering::Relaxed) {
        match cmd_rx.recv().await {
            Some(cmd) => {
                handle_command(&sftp, &events_tx, cmd).await;
            }
            None => break,
        }
    }

    let _ = sftp.close().await;
    alive.store(false, Ordering::Relaxed);
}

async fn handle_command(
    sftp: &SftpSession,
    events_tx: &mpsc::Sender<SftpEvent>,
    cmd: SftpCommand,
) {
    match cmd {
        SftpCommand::ListDir(path) => {
            match sftp.read_dir(&path).await {
                Ok(read_dir) => {
                    let entries: Vec<FileEntry> = read_dir
                        .filter_map(|e| {
                            let is_dir = e.file_type().is_dir();
                            let meta = e.metadata();
                            Some(FileEntry {
                                name: e.file_name(),
                                is_dir,
                                size: meta.size.unwrap_or(0),
                            })
                        })
                        .collect();
                    let _ = events_tx.send(SftpEvent::DirListing(entries));
                }
                Err(e) => {
                    let _ = events_tx.send(SftpEvent::Error(format!("List dir failed: {}", e)));
                }
            }
        }
        SftpCommand::Upload { local_path, remote_path } => {
            let result = match std::fs::read(&local_path) {
                Ok(data) => match sftp.write(&remote_path, &data).await {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("SFTP write failed: {}", e)),
                },
                Err(e) => Err(format!("Read local file failed: {}", e)),
            };
            let _ = events_tx.send(SftpEvent::UploadDone(result));
        }
        SftpCommand::Download { remote_path, local_path } => {
            let result = match sftp.read(&remote_path).await {
                Ok(data) => match std::fs::write(&local_path, &data) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("Write local file failed: {}", e)),
                },
                Err(e) => Err(format!("SFTP read failed: {}", e)),
            };
            let _ = events_tx.send(SftpEvent::DownloadDone(result));
        }
        SftpCommand::Mkdir(path) => {
            let result = sftp.create_dir(&path)
                .await
                .map_err(|e| format!("Mkdir failed: {}", e));
            let _ = events_tx.send(SftpEvent::MkdirDone(result));
        }
        SftpCommand::Delete { path, is_dir } => {
            let result = if is_dir {
                sftp.remove_dir(&path).await
            } else {
                sftp.remove_file(&path).await
            }.map_err(|e| format!("Delete failed: {}", e));
            let _ = events_tx.send(SftpEvent::DeleteDone(result));
        }
        SftpCommand::Disconnect => {}
    }
}
