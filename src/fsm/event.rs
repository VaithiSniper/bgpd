use crate::fsm::BGPState;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BGPEvent {
    OpenReceived,
    KeepAliveReceived,
    NotificationReceived,
    HoldTimerExpired,
    PeerDisconnected,
    LocalStart,
    SessionTerminated,
}

pub fn on_event(current_state: BGPState, event: BGPEvent) -> Result<BGPState, String> {
    match (current_state, event) {
        (BGPState::Idle, BGPEvent::LocalStart) => Ok(BGPState::OpenSent),
        (BGPState::Idle, BGPEvent::OpenReceived) => Ok(BGPState::OpenConfirm),
        (BGPState::OpenSent, BGPEvent::KeepAliveReceived) => Ok(BGPState::Established),
        (BGPState::OpenConfirm, BGPEvent::KeepAliveReceived) => Ok(BGPState::Established),
        (_, BGPEvent::PeerDisconnected) => Ok(BGPState::Idle),
        (_, BGPEvent::HoldTimerExpired) => Ok(BGPState::Idle),
        (_, BGPEvent::NotificationReceived) => Ok(BGPState::Idle),
        (_, BGPEvent::SessionTerminated) => Ok(BGPState::Idle),
        _ => Err(format!(
            "Invalid FSM transition: current_state={:?}, event={:?}",
            current_state, event
        )),
    }
}
