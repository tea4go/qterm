use russh::ChannelMsg;
use std::sync::mpsc::Sender;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc::Receiver;

use super::client;
use super::{SshConfig, SshError, SharedSshHandle};

/// SSH 会话主循环
/// 在后台线程中运行，处理终端数据读写、大小调整、会话生命周期
pub async fn run_ssh_session(
    config: SshConfig,
    rows: u16,
    cols: u16,
    output_tx: Sender<Vec<u8>>,          // 终端输出数据发送通道
    mut writer_rx: Receiver<Vec<u8>>,    // 终端输入数据接收通道
    mut resize_rx: Receiver<(u16, u16)>, // 大小调整请求接收通道
    alive: Arc<AtomicBool>,              // 会话存活标志
    handle_out: tokio::sync::oneshot::Sender<SharedSshHandle>, // russh 客户端句柄传递通道
) -> Result<(), SshError> {
    // 建立连接并认证
    let handle = client::connect_and_auth(&config).await?;
    let handle = Arc::new(tokio::sync::Mutex::new(handle));

    // 将 russh 客户端句柄传递给主线程（用于 SFTP 等）
    let _ = handle_out.send(handle.clone());

    // 打开 SSH 通道并请求 PTY 和 Shell
    let mut channel = {
        let h = handle.lock().await;
        h.channel_open_session()
            .await
            .map_err(|e| SshError::Channel(e.to_string()))?
    };

    // 请求伪终端（PTY），设置终端类型和初始大小
    channel
        .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    // 请求启动远程 Shell
    channel
        .request_shell(true)
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    // SSH 会话主循环：处理输出数据、输入数据、大小调整
    loop {
        if !alive.load(Ordering::Relaxed) {
            break;
        }

        tokio::select! {
            // 读取远程终端输出数据
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        // 将输出数据发送到主线程
                        if output_tx.send(data.to_vec()).is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | None => {
                        // 通道关闭或 EOF
                        break;
                    }
                    _ => {}
                }
            }
            // 写入用户输入数据到远程终端
            Some(data) = writer_rx.recv() => {
                if channel.data(&data[..]).await.is_err() {
                    break;
                }
            }
            // 处理终端大小调整请求
            Some((r, c)) = resize_rx.recv() => {
                let _ = channel.window_change(c as u32, r as u32, 0, 0).await;
            }
        }
    }

    // 会话结束：标记不存活，关闭通道
    alive.store(false, Ordering::Relaxed);
    let _ = channel.eof().await;
    let h = handle.lock().await;
    let _ = h.disconnect(russh::Disconnect::ByApplication, "", "").await;
    Ok(())
}