#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BGPState {
    Idle,
    Connect,
    OpenSent,
    OpenConfirm,
    Established,
}
