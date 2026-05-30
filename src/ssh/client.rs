use async_trait::async_trait;
use russh::client::{Config, Handler, Handle};
use russh::keys::key;
use std::sync::Arc;

/// SSH 客户端处理器
/// 实现 russh Handler 接口，目前自动接受所有服务器密钥
pub struct SshClient;

#[async_trait]
impl Handler for SshClient {
    type Error = russh::Error;

    /// 检查服务器公钥（当前自动接受所有密钥）
    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// 建立 SSH 连接并进行认证
/// 支持密码认证和私钥认证两种方式
pub async fn connect_and_auth(
    config: &super::SshConfig,
) -> Result<Handle<SshClient>, super::SshError> {
    let ssh_config = Arc::new(Config::default());
    let addr = format!("{}:{}", config.host, config.port);

    // 建立 TCP 连接
    let mut handle = russh::client::connect(ssh_config, &*addr, SshClient)
        .await
        .map_err(|e| super::SshError::Connection(e.to_string()))?;

    // 根据认证方式进行认证
    let authenticated = match &config.auth {
        super::SshAuth::Password(password) => {
            // 密码认证
            handle
                .authenticate_password(&config.username, password)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
        super::SshAuth::PrivateKey { path, passphrase } => {
            // 私钥认证：加载私钥文件
            let key = russh_keys::load_secret_key(path, passphrase.as_deref())
                .map_err(|e| super::SshError::Auth(format!("密钥加载错误: {}", e)))?;
            let key_pair = Arc::new(key);
            handle
                .authenticate_publickey(&config.username, key_pair)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
    };

    // 认证失败时返回错误
    if !authenticated {
        return Err(super::SshError::Auth("认证失败".to_string()));
    }

    Ok(handle)
}