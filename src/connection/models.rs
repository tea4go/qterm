use serde::Deserialize;

/// WhaleTerm connections.json 文件的顶层结构
#[derive(Deserialize)]
pub struct ConnectionsFile {
    pub groups: Vec<WhaleGroup>,
}

/// WhaleTerm 连接分组
#[derive(Deserialize)]
pub struct WhaleGroup {
    #[serde(rename = "groupName")]
    pub group_name: String,           // 分组名称
    pub connections: Vec<WhaleConnection>, // 分组中的连接列表
}

/// WhaleTerm 单个连接配置
#[derive(Deserialize)]
pub struct WhaleConnection {
    pub name: String,                 // 连接显示名称
    pub addr: String,                 // 主机地址
    pub port: u16,                    // SSH 端口
    pub username: String,             // 用户名
    pub password: String,             // 加密密码（AES-256-CFB）
    #[serde(rename = "authModel", default)]
    pub auth_model: String,           // 认证模型（password/key）
    #[serde(rename = "privateKey", default)]
    pub private_key: String,          // 私钥文件路径
}

/// QTerm 使用的简化连接结构体
/// 包含解密后的密码和分组名称
#[derive(Clone)]
pub struct Connection {
    pub name: String,          // 连接显示名称
    pub addr: String,          // 主机地址
    pub port: u16,             // SSH 端口
    pub username: String,      // 用户名
    pub password: String,      // 解密后的密码
    pub private_key: String,   // 私钥文件路径
    pub auth_model: String,    // 认证模型
    pub group_name: String,    // 所属分组名称
}