#[derive(Debug, Copy, Clone)]
pub enum BGPState {
    Idle,
    Connect,
    OpenSent,
    OpenConfirm,
    Established,
}
