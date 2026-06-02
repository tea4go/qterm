pub mod platform;

pub use platform::ShellType;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

/// 本地伪终端（PTY）句柄
/// 管理 Shell 进程的启动、数据读写、大小调整和关闭
pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,  // PTY 主端（用于读取和调整大小）
    writer: Box<dyn Write + Send>,      // PTY 写入端（用于向 Shell 发送数据）
    pub reader_rx: Receiver<Vec<u8>>,   // Shell 输出数据接收端
    child: Box<dyn portable_pty::Child + Send + Sync>,  // Shell 子进程
    stop_flag: Arc<AtomicBool>,         // 停止标志
}

impl PtyHandle {
    /// 启动本地伪终端
    /// 创建 PTY、启动 Shell 进程、启动数据读取线程
    pub fn spawn(rows: u16, cols: u16, shell: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        // 打开 PTY 主/从对
        let pair = pty_system.openpty(size)?;

        // 获取 Shell 命令路径（自定义或系统默认）
        let shell_cmd = shell
            .map(|s| s.to_string())
            .unwrap_or_else(platform::default_shell);

        // 构建 Shell 启动命令，设置终端类型和语言环境
        let mut cmd = CommandBuilder::new(&shell_cmd);
        cmd.env("TERM", "xterm-256color");
        #[cfg(windows)]
        {
            cmd.env("LANG", "en_US.UTF-8");
        }

        // 在 PTY 从端启动 Shell 子进程
        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        // 创建数据读取器和写入器
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        // 创建数据传输通道和停止标志
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop = stop_flag.clone();

        // 启动后台线程持续读取 Shell 输出数据
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,               // Shell 进程关闭
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;                  // 主线程已退出
                        }
                    }
                    Err(_) => break,               // 读取错误
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            reader_rx: rx,
            child,
            stop_flag,
        })
    }

    /// 向 Shell 进程写入数据
    pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// 调整 PTY 终端大小
    pub fn resize(&self, rows: u16, cols: u16) {
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let _ = self.master.resize(size);
    }

    /// 检查 Shell 进程是否仍在运行
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// 终止 Shell 进程
    pub fn kill(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
    }
}

impl Drop for PtyHandle {
    /// PTY 句柄销毁时自动终止 Shell 进程
    fn drop(&mut self) {
        self.kill();
    }
}