use serde::Deserialize;

/// Top-level structure of WhaleTerm's connections.json
#[derive(Deserialize)]
pub struct ConnectionsFile {
    pub groups: Vec<WhaleGroup>,
}

#[derive(Deserialize)]
pub struct WhaleGroup {
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub connections: Vec<WhaleConnection>,
}

#[derive(Deserialize)]
pub struct WhaleConnection {
    pub name: String,
    pub addr: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(rename = "authModel", default)]
    pub auth_model: String,
    #[serde(rename = "privateKey", default)]
    pub private_key: String,
}

/// Simplified connection for QTerm use
#[derive(Clone)]
pub struct Connection {
    pub name: String,
    pub addr: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub private_key: String,
    pub auth_model: String,
    pub group_name: String,
}
