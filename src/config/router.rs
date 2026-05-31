use crate::config::neighbor::NeighborConfig;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct RouterConfig {
    pub router_id: String,
    pub local_as: u16,
    pub listen_addr: String,
    pub keepalive_interval: u64,
    pub hold_interval: u64,
    pub neighbors: Vec<NeighborConfig>,
}

impl RouterConfig {
    pub fn load(path: &str) -> Result<Self, String> {
        let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&contents).map_err(|e| e.to_string())
    }
}
