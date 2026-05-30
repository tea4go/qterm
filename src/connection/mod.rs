pub mod models;

use std::path::PathBuf;

use self::models::{Connection, ConnectionsFile};

/// 获取 WhaleTerm 连接配置文件路径
/// Windows: %APPDATA%\WhaleTerm\connections.json
/// 其他: ~/.config/WhaleTerm/connections.json
fn whaleterm_config_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let mut p = PathBuf::from(appdata);
        p.push("WhaleTerm");
        p.push("connections.json");
        return p;
    }
    // 非 Windows 系统的回退路径
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("WhaleTerm");
        p.push("connections.json");
        return p;
    }
    PathBuf::from("connections.json")
}

/// 从 WhaleTerm 配置文件加载所有连接
/// 解析分组和连接信息，解密密码后返回扁平列表
pub fn load_connections() -> Vec<Connection> {
    let path = whaleterm_config_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let file: ConnectionsFile = match serde_json::from_str(&data) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    // 将分组中的连接展开为扁平列表
    let mut conns = Vec::new();
    for group in &file.groups {
        for wc in &group.connections {
            let password = decrypt_password(&wc.password);
            conns.push(Connection {
                name: wc.name.clone(),
                addr: wc.addr.clone(),
                port: wc.port,
                username: wc.username.clone(),
                password,
                private_key: wc.private_key.clone(),
                auth_model: wc.auth_model.clone(),
                group_name: group.group_name.clone(),
            });
        }
    }
    conns
}

/// AES-256-CFB 密码解密，兼容 WhaleTerm 加密格式
/// 格式：hex(IV[16字节] + ciphertext)
/// 密钥由主板序列号派生，或使用硬编码回退密钥
fn decrypt_password(hex_str: &str) -> String {
    use aes::Aes256;
    use cipher::{InnerIvInit, KeyInit};

    if hex_str.is_empty() {
        return String::new();
    }

    // 将十六进制字符串解码为字节
    let data = match hex::decode(hex_str) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    // 最小长度检查：16字节 IV + 至少1字节密文
    if data.len() < 17 {
        return String::new();
    }

    let key = derive_key();
    let (iv, ciphertext) = data.split_at(16);

    // 创建 AES-256 解密器
    let cipher = match Aes256::new_from_slice(&key) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    // 使用 CFB 模式解密
    let mut buffer = ciphertext.to_vec();
    let decryptor = cfb_mode::Decryptor::<Aes256>::inner_iv_slice_init(cipher, iv).unwrap();
    decryptor.decrypt(&mut buffer);

    String::from_utf8(buffer).unwrap_or_default()
}

/// 从主板序列号派生 32字节 AES 密钥，或使用硬编码回退密钥
/// 兼容 WhaleTerm 的 InitCommon() 密钥派生逻辑
fn derive_key() -> [u8; 32] {
    let fallback = b"51HytFKWhasDs2Q4E1mjHXQVJTm2SOym";

    let serial = get_motherboard_serial();

    match serial {
        Some(s) if !s.is_empty() => {
            let mut key = [0u8; 32];
            let serial_bytes = s.as_bytes();
            let serial_len = serial_bytes.len().min(32);
            let padded_len = serial_len.min(32);

            // 复制序列号字节（截断到32字节）
            key[..padded_len].copy_from_slice(&serial_bytes[..padded_len]);

            // 不足32字节时用回退密钥填充
            if padded_len < 32 {
                let pad_src = fallback;
                let mut offset = padded_len;
                let mut i = 0;
                while offset < 32 {
                    key[offset] = pad_src[i % pad_src.len()];
                    offset += 1;
                    i += 1;
                }
            }
            key
        }
        _ => {
            // 无主板序列号时使用硬编码回退密钥
            let mut key = [0u8; 32];
            key.copy_from_slice(&fallback[..32]);
            key
        }
    }
}

/// Windows: 通过 PowerShell 获取主板序列号
/// 用于 AES 密钥派生
fn get_motherboard_serial() -> Option<String> {
    let output = std::process::Command::new("powershell")
        .args(["-Command", "(Get-WmiObject Win32_BaseBoard).SerialNumber"])
        .output()
        .ok()?;
    let serial = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if serial.is_empty() { None } else { Some(serial) }
}