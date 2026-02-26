use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub node_name: String,
    pub listen_address: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub data_dir: String,
    pub validation_seed: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_name: "call-core-node".to_string(),
            listen_address: "0.0.0.0:51235".parse().unwrap(),
            peers: Vec::new(),
            data_dir: "./data".to_string(),
            validation_seed: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file(_path: &str) -> anyhow::Result<Self> {
        Ok(Self::default())
    }
}
