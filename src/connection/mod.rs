pub mod models;

use std::path::PathBuf;

use self::models::{Connection, ConnectionsFile};

fn whaleterm_config_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let mut p = PathBuf::from(appdata);
        p.push("WhaleTerm");
        p.push("connections.json");
        return p;
    }
    // Fallback for non-Windows
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("WhaleTerm");
        p.push("connections.json");
        return p;
    }
    PathBuf::from("connections.json")
}

/// Load all connections from the WhaleTerm config file.
/// Returns a flat list of connections with their group names.
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

/// AES-256-CFB decryption, compatible with WhaleTerm.
/// Format: hex(IV[16 bytes] + ciphertext)
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

    if data.len() < 17 { // minimum: 16 IV + 1 byte ciphertext
        return String::new();
    }

    let key = derive_key();
    let (iv, ciphertext) = data.split_at(16);

    let cipher = match Aes256::new_from_slice(&key) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut buffer = ciphertext.to_vec();
    let decryptor = cfb_mode::Decryptor::<Aes256>::inner_iv_slice_init(cipher, iv).unwrap();
    decryptor.decrypt(&mut buffer);

    String::from_utf8(buffer).unwrap_or_default()
}

/// Derive 32-byte key from motherboard serial, or use fallback.
/// Mirrors WhaleTerm's InitCommon() logic.
fn derive_key() -> [u8; 32] {
    let fallback = b"51HytFKWhasDs2Q4E1mjHXQVJTm2SOym";

    let serial = get_motherboard_serial();

    match serial {
        Some(s) if !s.is_empty() => {
            let mut key = [0u8; 32];
            let serial_bytes = s.as_bytes();
            let serial_len = serial_bytes.len().min(32);
            let padded_len = serial_len.min(32);

            // Copy serial bytes (truncated to 32)
            key[..padded_len].copy_from_slice(&serial_bytes[..padded_len]);

            // Pad with fallback string
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
            let mut key = [0u8; 32];
            key.copy_from_slice(&fallback[..32]);
            key
        }
    }
}

/// Windows: get motherboard serial via PowerShell
fn get_motherboard_serial() -> Option<String> {
    let output = std::process::Command::new("powershell")
        .args(["-Command", "(Get-WmiObject Win32_BaseBoard).SerialNumber"])
        .output()
        .ok()?;
    let serial = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if serial.is_empty() { None } else { Some(serial) }
}
