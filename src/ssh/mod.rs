pub mod client;
pub mod session;

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};
use tokio::runtime::Runtime;

static SSH_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    SSH_RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create tokio runtime")
    })
}

#[derive(Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub timeout_secs: u32,
}

#[derive(Clone)]
pub enum SshAuth {
    Password(String),
    PrivateKey { path: String, passphrase: Option<String> },
}

#[derive(Debug)]
pub enum SshError {
    Connection(String),
    Auth(String),
    Channel(String),
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::Connection(e) => write!(f, "Connection error: {}", e),
            SshError::Auth(e) => write!(f, "Auth error: {}", e),
            SshError::Channel(e) => write!(f, "Channel error: {}", e),
        }
    }
}

impl std::error::Error for SshError {}

pub struct SshHandle {
    pub reader_rx: Receiver<Vec<u8>>,
    writer_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    resize_tx: tokio::sync::mpsc::Sender<(u16, u16)>,
    alive: Arc<AtomicBool>,
}

impl SshHandle {
    pub fn connect(config: SshConfig, rows: u16, cols: u16) -> Result<Self, SshError> {
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();
        let (writer_tx, writer_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
        let (resize_tx, resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(16);
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        let rt = get_runtime();
        let config_clone = config.clone();
        let alive_spawn = alive.clone();

        std::thread::spawn(move || {
            rt.block_on(async move {
                match session::run_ssh_session(
                    config_clone, rows, cols, output_tx, writer_rx, resize_rx, alive_spawn,
                ).await {
                    Ok(()) => {}
                    Err(e) => eprintln!("SSH session error: {}", e),
                }
            });
            alive_clone.store(false, Ordering::Relaxed);
        });

        Ok(Self {
            reader_rx: output_rx,
            writer_tx,
            resize_tx,
            alive,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<(), SshError> {
        let _ = self.writer_tx.try_send(data.to_vec());
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.resize_tx.try_send((rows, cols));
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn disconnect(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}
