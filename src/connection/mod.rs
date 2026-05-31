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

    let data = match hex::decode(hex_str) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    if data.len() < 17 {
        return String::new();
    }

    // 使用缓存的密钥，避免重复调用
    static KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    let key = KEY.get_or_init(derive_key_uncached);

    let (iv, ciphertext) = data.split_at(16);

    let cipher = match Aes256::new_from_slice(key) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut buffer = ciphertext.to_vec();
    let decryptor = cfb_mode::Decryptor::<Aes256>::inner_iv_slice_init(cipher, iv).unwrap();
    decryptor.decrypt(&mut buffer);

    String::from_utf8(buffer).unwrap_or_default()
}

/// 从主板序列号派生 32字节 AES 密钥，或使用硬编码回退密钥
/// 兼容 WhaleTerm 的 InitCommon() 密钥派生逻辑
fn derive_key_uncached() -> [u8; 32] {
    let fallback = b"51HytFKWhasDs2Q4E1mjHXQVJTm2SOym";

    let serial = get_motherboard_serial();

    match serial {
        Some(s) if !s.is_empty() => {
            let mut key = [0u8; 32];
            let serial_bytes = s.as_bytes();
            let padded_len = serial_bytes.len().min(32);

            key[..padded_len].copy_from_slice(&serial_bytes[..padded_len]);

            if padded_len < 32 {
                let mut offset = padded_len;
                let mut i = 0;
                while offset < 32 {
                    key[offset] = fallback[i % fallback.len()];
                    offset += 1;
                    i += 1;
                }
            }
            key
        }
        _ => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&fallback[..32]);
            key
        }
    }
}

/// 通过 Win32 原生 API 读取 SMBIOS Type 2 (Baseboard) 序列号
/// 无需启动任何外部进程
#[cfg(windows)]
fn get_motherboard_serial() -> Option<String> {
    const RSMB: u32 = u32::from_le_bytes([b'R', b'S', b'M', b'B']);

    extern "system" {
        fn GetSystemFirmwareTable(
            provider: u32,
            id: u32,
            buf: *mut u8,
            size: u32,
        ) -> u32;
    }

    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
    if size == 0 {
        return None;
    }

    let mut buf = vec![0u8; size as usize];
    let written = unsafe { GetSystemFirmwareTable(RSMB, 0, buf.as_mut_ptr(), size) };
    if written == 0 {
        return None;
    }

    // 跳过 8 字节 RawSMBIOSData 头部
    if buf.len() < 8 {
        return None;
    }
    let data = &buf[8..];

    // 遍历 SMBIOS 结构查找 Type 2 (Baseboard Information)
    let mut off = 0usize;
    while off + 4 <= data.len() {
        let stype = data[off];
        let slen = data[off + 1] as usize;

        if stype == 2 && off + slen <= data.len() && slen > 7 {
            let serial_idx = data[off + 7] as usize;
            if serial_idx > 0 {
                let strings = &data[off + slen..];
                if let Some(s) = smbios_string(strings, serial_idx) {
                    return Some(s);
                }
            }
        }

        // 跳过格式化区域
        off += slen;
        // 跳过字符串区域（双 \0 结尾）
        while off + 1 < data.len() && !(data[off] == 0 && data[off + 1] == 0) {
            off += 1;
        }
        off += 2;
    }

    None
}

/// 从 SMBIOS 字符串表中按索引提取字符串
fn smbios_string(data: &[u8], index: usize) -> Option<String> {
    if index == 0 {
        return None;
    }
    let mut cur = 1;
    let mut start = 0;
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0 {
            if cur == index {
                let s = String::from_utf8_lossy(&data[start..i]).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
                return None;
            }
            cur += 1;
            start = i + 1;
            // 双 \0 = 字符串表结束
            if i + 1 < data.len() && data[i + 1] == 0 {
                break;
            }
        }
        i += 1;
    }
    None
}

#[cfg(not(windows))]
fn get_motherboard_serial() -> Option<String> {
    None
}