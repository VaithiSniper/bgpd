use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BGPState {
    Idle,
    Connect,
    OpenSent,
    OpenConfirm,
    Established,
}

impl fmt::Display for BGPState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BGPState::Idle => write!(f, "Idle"),
            BGPState::Connect => write!(f, "Connect"),
            BGPState::OpenSent => write!(f, "OpenSent"),
            BGPState::OpenConfirm => write!(f, "OpenConfirm"),
            BGPState::Established => write!(f, "Established"),
        }
    }
}
