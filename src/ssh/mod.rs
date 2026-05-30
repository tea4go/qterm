pub mod client;
pub mod session;

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};
use tokio::runtime::Runtime;

/// 全局 SSH 专用的 tokio 运行时（懒初始化）
static SSH_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// 获取或创建 SSH 专用的 tokio 运行时
pub fn get_runtime() -> &'static Runtime {
    SSH_RUNTIME.get_or_init(|| {
        Runtime::new().expect("创建 tokio 运行时失败")
    })
}

/// SSH 连接配置
#[derive(Clone)]
pub struct SshConfig {
    pub host: String,          // 主机地址
    pub port: u16,             // SSH 端口
    pub username: String,      // 用户名
    pub auth: SshAuth,         // 认证方式
    pub timeout_secs: u32,     // 连接超时秒数
}

/// SSH 认证方式
#[derive(Clone)]
pub enum SshAuth {
    Password(String),                           // 密码认证
    PrivateKey { path: String, passphrase: Option<String> }, // 私钥认证
}

/// SSH 错误类型
#[derive(Debug)]
pub enum SshError {
    Connection(String),  // 连接错误
    Auth(String),        // 认证错误
    Channel(String),     // 通道错误
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::Connection(e) => write!(f, "连接错误: {}", e),
            SshError::Auth(e) => write!(f, "认证错误: {}", e),
            SshError::Channel(e) => write!(f, "通道错误: {}", e),
        }
    }
}

impl std::error::Error for SshError {}

/// 共享的 russh 客户端句柄（用于 SFTP 等子系统复用）
pub type SharedSshHandle = Arc<tokio::sync::Mutex<russh::client::Handle<client::SshClient>>>;

/// SSH 连接句柄
/// 管理 SSH 会话的数据读写、终端大小调整、SFTP 子系统等
pub struct SshHandle {
    pub reader_rx: Receiver<Vec<u8>>,            // 终端输出数据接收端
    pub writer_tx: tokio::sync::mpsc::Sender<Vec<u8>>,  // 终端输入数据发送端
    pub resize_tx: tokio::sync::mpsc::Sender<(u16, u16)>, // 终端大小调整发送端
    alive: Arc<AtomicBool>,                      // 连接存活标志
    russh_handle: SharedSshHandle,               // russh 客户端句柄（用于 SFTP）
}

impl SshHandle {
    /// 建立 SSH 连接并创建终端会话
    /// 启动后台线程运行 SSH 会话循环
    pub fn connect(config: SshConfig, rows: u16, cols: u16) -> Result<Self, SshError> {
        // 创建数据通道：输出、输入、大小调整、句柄传递
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();
        let (writer_tx, writer_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
        let (resize_tx, resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(16);
        let (handle_tx, handle_rx) = tokio::sync::oneshot::channel::<SharedSshHandle>();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        let rt = get_runtime();
        let config_clone = config.clone();
        let alive_spawn = alive.clone();

        // 在后台线程中运行 SSH 会话
        std::thread::spawn(move || {
            rt.block_on(async move {
                match session::run_ssh_session(
                    config_clone, rows, cols, output_tx, writer_rx, resize_rx, alive_spawn, handle_tx,
                ).await {
                    Ok(()) => {}
                    Err(e) => eprintln!("SSH 会话错误: {}", e),
                }
            });
            // 会话结束后标记连接为不存活
            alive_clone.store(false, Ordering::Relaxed);
        });

        // 等待 russh 客户端句柄传递
        let russh_handle = handle_rx.blocking_recv()
            .map_err(|_| SshError::Channel("初始化 SSH 会话失败".to_string()))?;

        Ok(Self {
            reader_rx: output_rx,
            writer_tx,
            resize_tx,
            alive,
            russh_handle,
        })
    }

    /// 向 SSH 终端写入数据
    pub fn write(&self, data: &[u8]) -> Result<(), SshError> {
        let _ = self.writer_tx.try_send(data.to_vec());
        Ok(())
    }

    /// 请求调整远程终端大小
    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.resize_tx.try_send((rows, cols));
    }

    /// 检查 SSH 连接是否存活
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// 断开 SSH 连接
    pub fn disconnect(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }

    /// 从当前 SSH 连接打开 SFTP 子系统
    pub fn open_sftp(&self) -> Result<crate::sftp::SftpHandle, SshError> {
        crate::sftp::SftpHandle::new(self.russh_handle.clone(), get_runtime())
    }
}