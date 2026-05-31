use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NeighborConfig {
    pub address: String,
    pub peer_as: u32,
    pub passive: bool,
}
