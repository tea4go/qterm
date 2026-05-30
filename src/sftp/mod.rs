use std::sync::{Arc, mpsc::{self, Receiver}, atomic::{AtomicBool, Ordering}};
use tokio::runtime::Runtime;
use russh_sftp::client::SftpSession;

use crate::ssh::SharedSshHandle;

/// SFTP 客户端句柄
/// 通过命令通道和事件通道与后台 SFTP 任务通信
pub struct SftpHandle {
    events_rx: Receiver<SftpEvent>,            // SFTP 事件接收端
    cmd_tx: tokio::sync::mpsc::Sender<SftpCommand>,  // SFTP 命令发送端
    alive: Arc<AtomicBool>,                    // 连接存活标志
}

/// 远程文件条目（目录/文件信息）
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,      // 文件名
    pub is_dir: bool,      // 是否为目录
    pub size: u64,         // 文件大小
}

/// SFTP 事件类型
/// 从后台 SFTP 任务发送到主线程的通知
pub enum SftpEvent {
    Connected,                   // 连接成功
    DirListing(Vec<FileEntry>),  // 目录列表结果
    UploadDone(Result<(), String>),  // 上传完成
    DownloadDone(Result<(), String>), // 下载完成
    MkdirDone(Result<(), String>),   // 创建目录完成
    DeleteDone(Result<(), String>),  // 删除完成
    Error(String),                // 错误
}

/// SFTP 命令类型
/// 从主线程发送到后台 SFTP 任务的操作指令
enum SftpCommand {
    ListDir(String),                              // 列出目录
    Upload { local_path: String, remote_path: String },  // 上传文件
    Download { remote_path: String, local_path: String }, // 下载文件
    Mkdir(String),                                 // 创建目录
    Delete { path: String, is_dir: bool },          // 删除文件/目录
    Disconnect,                                     // 断开连接
}

impl SftpHandle {
    /// 创建 SFTP 连接
    /// 从现有 SSH 连接句柄和 tokio 运行时初始化 SFTP 会话
    pub fn new(
        ssh_handle: SharedSshHandle,
        rt: &Runtime,
    ) -> Result<Self, crate::ssh::SshError> {
        let (events_tx, events_rx) = mpsc::channel::<SftpEvent>();
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<SftpCommand>(256);
        let alive = Arc::new(AtomicBool::new(true));

        let alive_clone = alive.clone();
        // 在 tokio 运行时上启动后台 SFTP 任务
        rt.spawn(async move {
            sftp_task(ssh_handle, events_tx, cmd_rx, alive_clone).await;
        });

        Ok(Self {
            events_rx,
            cmd_tx,
            alive,
        })
    }

    /// 轮询 SFTP 事件（非阻塞）
    /// 从事件通道中取出所有可用事件
    pub fn poll(&self) -> Vec<SftpEvent> {
        let mut events = Vec::new();
        while let Ok(e) = self.events_rx.try_recv() {
            events.push(e);
        }
        events
    }

    /// 检查 SFTP 连接是否存活
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// 请求列出指定目录的文件
    pub fn list_dir(&self, path: &str) {
        let _ = self.cmd_tx.try_send(SftpCommand::ListDir(path.to_string()));
    }

    /// 请求上传文件（本地 → 远程）
    pub fn upload(&self, local_path: String, remote_path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Upload { local_path, remote_path });
    }

    /// 请求下载文件（远程 → 本地）
    pub fn download(&self, remote_path: String, local_path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Download { remote_path, local_path });
    }

    /// 请求在远程创建目录
    pub fn mkdir(&self, path: String) {
        let _ = self.cmd_tx.try_send(SftpCommand::Mkdir(path));
    }

    /// 请求删除远程文件或目录
    pub fn delete(&self, path: String, is_dir: bool) {
        let _ = self.cmd_tx.try_send(SftpCommand::Delete { path, is_dir });
    }

    /// 断开 SFTP 连接
    pub fn disconnect(&self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.cmd_tx.try_send(SftpCommand::Disconnect);
    }
}

/// 后台 SFTP 任务
/// 初始化 SFTP 会话后循环处理命令
async fn sftp_task(
    ssh_handle: SharedSshHandle,
    events_tx: mpsc::Sender<SftpEvent>,
    mut cmd_rx: tokio::sync::mpsc::Receiver<SftpCommand>,
    alive: Arc<AtomicBool>,
) {
    // 通过 SSH 连接打开 SFTP 子系统通道
    let sftp = {
        let h = ssh_handle.lock().await;
        match h.channel_open_session().await {
            Ok(channel) => {
                // 请求 SFTP 子系统
                if let Err(e) = channel.request_subsystem(true, "sftp").await {
                    let _ = events_tx.send(SftpEvent::Error(format!("SFTP 子系统请求失败: {}", e)));
                    return;
                }
                // 创建 SFTP 会话
                match SftpSession::new(channel.into_stream()).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = events_tx.send(SftpEvent::Error(format!("SFTP 会话初始化失败: {}", e)));
                        return;
                    }
                }
            }
            Err(e) => {
                let _ = events_tx.send(SftpEvent::Error(format!("打开 SFTP 通道失败: {}", e)));
                return;
            }
        }
    };

    // 通知主线程连接成功
    let _ = events_tx.send(SftpEvent::Connected);

    // 命令处理循环
    while alive.load(Ordering::Relaxed) {
        match cmd_rx.recv().await {
            Some(cmd) => {
                handle_command(&sftp, &events_tx, cmd).await;
            }
            None => break,
        }
    }

    // 关闭 SFTP 会话
    let _ = sftp.close().await;
    alive.store(false, Ordering::Relaxed);
}

/// 处理单个 SFTP 命令
async fn handle_command(
    sftp: &SftpSession,
    events_tx: &mpsc::Sender<SftpEvent>,
    cmd: SftpCommand,
) {
    match cmd {
        SftpCommand::ListDir(path) => {
            // 列出远程目录内容
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
                    let _ = events_tx.send(SftpEvent::Error(format!("列出目录失败: {}", e)));
                }
            }
        }
        SftpCommand::Upload { local_path, remote_path } => {
            // 上传文件：读取本地文件 → 写入远程
            let result = match std::fs::read(&local_path) {
                Ok(data) => match sftp.write(&remote_path, &data).await {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("SFTP 写入失败: {}", e)),
                },
                Err(e) => Err(format!("读取本地文件失败: {}", e)),
            };
            let _ = events_tx.send(SftpEvent::UploadDone(result));
        }
        SftpCommand::Download { remote_path, local_path } => {
            // 下载文件：读取远程文件 → 写入本地
            let result = match sftp.read(&remote_path).await {
                Ok(data) => match std::fs::write(&local_path, &data) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("写入本地文件失败: {}", e)),
                },
                Err(e) => Err(format!("SFTP 读取失败: {}", e)),
            };
            let _ = events_tx.send(SftpEvent::DownloadDone(result));
        }
        SftpCommand::Mkdir(path) => {
            // 创建远程目录
            let result = sftp.create_dir(&path)
                .await
                .map_err(|e| format!("创建目录失败: {}", e));
            let _ = events_tx.send(SftpEvent::MkdirDone(result));
        }
        SftpCommand::Delete { path, is_dir } => {
            // 删除远程文件或目录
            let result = if is_dir {
                sftp.remove_dir(&path).await
            } else {
                sftp.remove_file(&path).await
            }.map_err(|e| format!("删除失败: {}", e));
            let _ = events_tx.send(SftpEvent::DeleteDone(result));
        }
        SftpCommand::Disconnect => {}
    }
}