use crate::bgp::timers::{TimerConfig, Timers};
use crate::fsm::BGPState;
use crate::fsm::event::BGPEvent;
use crate::net::Peer;
use crate::packet::{BGPMessage, NotificationErrorCode, NotificationMessage, OpenMessage};
use crate::util::{log_debug, log_error, log_warn};
use crate::{fsm, util};
use std::sync::mpsc;

const MODULE: &str = "bgp.session";

pub enum SessionEvent {
    MessageReceived(BGPMessage),
    HoldTimerRefresh,
    HoldTimerExpired,
    KeepAliveTimerExpired,
    PeerDisconnected(),
}

#[derive(Debug, Clone)]
pub struct SessionOpts {
    router_id: String,
    local_as: u16,
    pub enable_timer_monitors: bool,
}

impl SessionOpts {
    pub fn new(router_id: String, local_as: u16, enable_timer_monitors: bool) -> SessionOpts {
        SessionOpts {
            router_id,
            local_as,
            enable_timer_monitors,
        }
    }
}

#[derive(Debug, Default)]
pub struct SessionNegotiation {
    pub open_sent: bool,
    pub open_received: bool,

    pub peer_asn: u16,
    pub peer_router_id: u32,
}

pub struct Session {
    pub peer: Peer,
    state: BGPState,
    pub timers: Timers,
    pub tx_event_chan: mpsc::Sender<SessionEvent>,
    pub rx_event_chan: mpsc::Receiver<SessionEvent>,
    pub opts: SessionOpts,
    pub negotiation: SessionNegotiation,
}

impl Session {
    pub fn new(cfg: SessionOpts, timer_cfg: TimerConfig, peer: Peer) -> Self {
        let (tx, rx) = mpsc::channel::<SessionEvent>();
        Self {
            peer,
            state: BGPState::Idle,
            timers: Timers::new(timer_cfg),
            tx_event_chan: tx,
            rx_event_chan: rx,
            opts: cfg,
            negotiation: SessionNegotiation::default(),
        }
    }

    pub fn apply_fsm_event(&mut self, event: BGPEvent) -> Result<(), String> {
        let next_state = fsm::on_event(self.state, event)?;
        log_debug(
            MODULE,
            "Got BGP message",
            &[("type", "NOTIFICATION".to_string())],
        );
        log_debug(
            MODULE,
            "Applying FSM transition",
            &[
                ("peer", self.peer.get_ip().to_string()),
                ("from", self.state.to_string()),
                ("to", next_state.to_string()),
            ],
        );
        self.state = next_state;
        Ok(())
    }

    pub fn is_established(&self) -> bool {
        self.state == BGPState::Established
    }

    pub fn initiate(&mut self) -> Result<(), String> {
        self.send_open()?;
        self.apply_fsm_event(BGPEvent::LocalStart)?;
        Ok(())
    }

    pub fn teardown(&mut self) -> Result<(), String> {
        log_warn(
            MODULE,
            "Tearing down connection with peer",
            &[("peer", self.peer.get_ip().to_string())],
        );
        self.peer.close()?;
        self.apply_fsm_event(BGPEvent::PeerDisconnected)?;

        Ok(())
    }

    pub fn run(&mut self) {
        // Start reader/writer threads and go into event loop
        let tx_clone_reader = self.tx_event_chan.clone();
        self.start_reader_thread(tx_clone_reader);
        loop {
            let event = self.rx_event_chan.recv().unwrap();
            if let Err(e) = self.dispatch_event_handler(event) {
                log_error(
                    MODULE,
                    "Terminating session due to error",
                    &[("error", e.to_string())],
                );
                break;
            }
        }
    }

    pub fn dispatch_event_handler(&mut self, event: SessionEvent) -> Result<(), String> {
        match event {
            SessionEvent::MessageReceived(msg) => self.handle_msg(msg),
            SessionEvent::KeepAliveTimerExpired => self.handle_keepalive_expiry(),
            SessionEvent::HoldTimerExpired => self.handle_hold_expiry(),
            SessionEvent::HoldTimerRefresh => self.handle_hold_refresh(),
            SessionEvent::PeerDisconnected() => self.handle_peer_disconnect(),
        }
    }

    fn send_open(&mut self) -> Result<(), String> {
        let open_msg = OpenMessage {
            version: 4,
            asn: self.opts.local_as,
            hold_time: self.timers.local_cfg.hold_interval.as_secs() as u16,
            bgp_id: util::ipv4_str_to_u32(&self.opts.router_id)?,
            opt_len: 0,
            opts: Vec::new(),
        };
        log_debug(
            MODULE,
            "Sending OPEN message",
            &[("to", self.peer.get_ip().to_string())],
        );
        let bgp_msg = BGPMessage::Open(open_msg);
        self.peer.send_message(bgp_msg).map_err(|e| e.to_string())?;

        self.negotiation.open_sent = true;

        Ok(())
    }

    fn handle_msg(&mut self, msg: BGPMessage) -> Result<(), String> {
        match msg {
            BGPMessage::Open(open) => {
                log_debug(
                    MODULE,
                    "Got BGP message from peer",
                    &[
                        ("type", "OPEN".to_string()),
                        ("from", self.peer.get_ip().to_string()),
                    ],
                );
                self.negotiation.open_received = true;
                // For OPEN:
                // - Validate peer metadata and store
                // - If not sent OPEN yet, send it out now
                // - Negotiate timers
                // - Transition to OpenConfirm
                // - Send KeepAlive

                self.negotiation.peer_asn = open.asn;
                self.negotiation.peer_router_id = open.bgp_id;

                if !self.negotiation.open_sent {
                    self.send_open()?;
                }
                self.timers.negotiate(open.hold_time);

                self.apply_fsm_event(BGPEvent::OpenReceived)?;

                log_debug(
                    MODULE,
                    "Sending KEEPALIVE after OPEN received",
                    &[("to", self.peer.get_ip().to_string())],
                );
                self.peer.send_message(BGPMessage::KeepAlive)
            }
            BGPMessage::KeepAlive => {
                log_debug(
                    MODULE,
                    "Got BGP message from peer",
                    &[
                        ("type", "KEEPALIVE".to_string()),
                        ("from", self.peer.get_ip().to_string()),
                    ],
                );
                // For KEEPALIVE:
                // - Transition to Established
                // - If first time transitioning into establishing:
                //     - Start timers
                // - Refresh hold timer
                let was_established = self.is_established();
                self.apply_fsm_event(BGPEvent::KeepAliveReceived)?;
                if !was_established {
                    // If it is freshly established, setup timers in threads
                    self.start_timer_threads();
                }
                self.tx_event_chan
                    .send(SessionEvent::HoldTimerRefresh)
                    .map_err(|e| e.to_string())
            }
            BGPMessage::Notification(notification) => {
                log_debug(
                    MODULE,
                    "Got BGP message from peer",
                    &[
                        ("type", "NOTIFICATION".to_string()),
                        ("err_code", notification.err_code.to_string()),
                        ("from", self.peer.get_ip().to_string()),
                    ],
                );
                // For NOTIFICATION:
                // - Check error code
                // - Most cases require session teardown
                self.teardown()?;
                Err(format!("Received notification {:?}", notification.err_code))
            }
        }
    }

    pub fn handle_keepalive_expiry(&mut self) -> Result<(), String> {
        log_debug(
            MODULE,
            "KEEPALIVE timer expired, sending new message",
            &[
                (
                    "interval",
                    self.timers
                        .negotiated_cfg
                        .keepalive_interval
                        .as_secs()
                        .to_string(),
                ),
                ("to", self.peer.get_ip().to_string()),
            ],
        );
        self.peer.send_message(BGPMessage::KeepAlive)?;
        self.timers.update_last_keepalive_tx();
        Ok(())
    }

    pub fn handle_hold_expiry(&mut self) -> Result<(), String> {
        log_error(
            MODULE,
            "HOLD timer expired, sending NOTIFICATION message",
            &[
                (
                    "interval",
                    self.timers
                        .negotiated_cfg
                        .hold_interval
                        .as_secs()
                        .to_string(),
                ),
                ("to", self.peer.get_ip().to_string()),
                ("type", "NOTIFICATION".to_string()),
                (
                    "err_code",
                    NotificationErrorCode::HoldTimerExpired.to_string(),
                ),
            ],
        );
        let notification_msg =
            NotificationMessage::new(NotificationErrorCode::HoldTimerExpired, 0, Vec::new());
        self.peer
            .send_message(BGPMessage::Notification(notification_msg))?;
        log_error(
            MODULE,
            "HOLD timer expired, tearing down session",
            &[
                (
                    "interval",
                    self.timers
                        .negotiated_cfg
                        .hold_interval
                        .as_secs()
                        .to_string(),
                ),
                ("peer", self.peer.get_ip().to_string()),
            ],
        );
        self.teardown()?;
        Err("Hold timer expired, terminating session".to_string())
    }

    pub fn handle_hold_refresh(&mut self) -> Result<(), String> {
        log_debug(
            MODULE,
            "Refreshed hold timer",
            &[
                ("peer", self.peer.get_ip().to_string()),
                (
                    "interval",
                    self.timers
                        .negotiated_cfg
                        .hold_interval
                        .as_secs()
                        .to_string(),
                ),
            ],
        );
        self.timers.update_last_keepalive_rx();
        Ok(())
    }

    pub fn handle_peer_disconnect(&mut self) -> Result<(), String> {
        log_error(
            MODULE,
            "Peer disconnected",
            &[("peer", self.peer.get_ip().to_string())],
        );
        self.teardown()?;
        Err("Peer disconnected, terminating session".to_string())
    }

    pub fn start_reader_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        log_debug(
            MODULE,
            "Starting reader thread",
            &[("peer", self.peer.get_ip().to_string())],
        );
        let mut peer_reader = self.peer.clone_reader().unwrap();
        std::thread::spawn(move || {
            loop {
                match peer_reader.recv_message() {
                    Ok(msg) => {
                        if tx_event_chan
                            .send(SessionEvent::MessageReceived(msg))
                            .is_err()
                        {
                            log_error(MODULE, "Session already gone", &[]);
                            break;
                        }
                    }
                    Err(e) => {
                        log_error(
                            MODULE,
                            "Error receiving message from peer",
                            &[("error", e.to_string())],
                        );
                        if tx_event_chan
                            .send(SessionEvent::PeerDisconnected())
                            .is_err()
                        {
                            log_error(MODULE, "Session already gone", &[("error", e.to_string())]);
                        }
                        break;
                    }
                }
            }
        });
    }

    pub fn start_timer_threads(&mut self) {
        // - KEEPALIVE timer: Track keepalive interval. On expiry, we should send KEEPALIVE message
        // - HOLD timer: Track hold interval. On expiry, we should tear down session.
        let tx_clone_keepalive_timer = self.tx_event_chan.clone();
        self.timers
            .start_keepalive_timer_thread(tx_clone_keepalive_timer);
        let tx_clone_hold_timer = self.tx_event_chan.clone();
        self.timers.start_hold_timer_thread(tx_clone_hold_timer);

        if self.opts.enable_timer_monitors {
            // - KEEPALIVE monitor: To dump value of keepalive timer
            // - HOLD monitor: To dump value of hold timer
            self.timers.start_hold_monitor();
            self.timers.start_keepalive_monitor();
        }
    }
}
