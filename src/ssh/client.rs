use async_trait::async_trait;
use russh::client::{Config, Handler, Handle};
use russh::keys::key;
use std::sync::Arc;

pub struct SshClient;

#[async_trait]
impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub async fn connect_and_auth(
    config: &super::SshConfig,
) -> Result<Handle<SshClient>, super::SshError> {
    let ssh_config = Arc::new(Config::default());
    let addr = format!("{}:{}", config.host, config.port);

    let mut handle = russh::client::connect(ssh_config, &*addr, SshClient)
        .await
        .map_err(|e| super::SshError::Connection(e.to_string()))?;

    let authenticated = match &config.auth {
        super::SshAuth::Password(password) => {
            handle
                .authenticate_password(&config.username, password)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
        super::SshAuth::PrivateKey { path, passphrase } => {
            let key = russh_keys::load_secret_key(path, passphrase.as_deref())
                .map_err(|e| super::SshError::Auth(format!("Key load error: {}", e)))?;
            let key_pair = Arc::new(key);
            handle
                .authenticate_publickey(&config.username, key_pair)
                .await
                .map_err(|e| super::SshError::Auth(e.to_string()))?
        }
    };

    if !authenticated {
        return Err(super::SshError::Auth("Authentication failed".to_string()));
    }

    Ok(handle)
}
