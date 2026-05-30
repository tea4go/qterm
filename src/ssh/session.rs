use russh::ChannelMsg;
use std::sync::mpsc::Sender;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc::Receiver;

use super::client;
use super::{SshConfig, SshError};

pub async fn run_ssh_session(
    config: SshConfig,
    rows: u16,
    cols: u16,
    output_tx: Sender<Vec<u8>>,
    mut writer_rx: Receiver<Vec<u8>>,
    mut resize_rx: Receiver<(u16, u16)>,
    alive: Arc<AtomicBool>,
) -> Result<(), SshError> {
    let handle = client::connect_and_auth(&config).await?;

    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    channel
        .request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    channel
        .request_shell(true)
        .await
        .map_err(|e| SshError::Channel(e.to_string()))?;

    loop {
        if !alive.load(Ordering::Relaxed) {
            break;
        }

        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        if output_tx.send(data.to_vec()).is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | None => {
                        break;
                    }
                    _ => {}
                }
            }
            Some(data) = writer_rx.recv() => {
                if channel.data(&data[..]).await.is_err() {
                    break;
                }
            }
            Some((r, c)) = resize_rx.recv() => {
                let _ = channel.window_change(c as u32, r as u32, 0, 0).await;
            }
        }
    }

    alive.store(false, Ordering::Relaxed);
    let _ = channel.eof().await;
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "").await;
    Ok(())
}
