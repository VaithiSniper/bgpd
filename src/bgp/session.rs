use crate::bgp::timers::{TimerOpts, Timers};
use crate::fsm::event::BGPEvent;
use crate::fsm::BGPState;
use crate::net::Peer;
use crate::packet::{BGPMessage, NotificationErrorCode, NotificationMessage, OpenMessage};
use crate::{fsm, util};
use std::sync::mpsc;
use std::time::Duration;

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
}
impl SessionOpts {
    pub fn new(router_id: String, local_as: u16) -> SessionOpts {
        SessionOpts {
            router_id,
            local_as,
        }
    }
}

pub struct Session {
    pub peer: Peer,
    state: BGPState,
    pub timers: Timers,
    pub tx_event_chan: mpsc::Sender<SessionEvent>,
    pub rx_event_chan: mpsc::Receiver<SessionEvent>,
    pub opts: SessionOpts,
}

impl Session {
    pub fn new(cfg: SessionOpts, timer_cfg: TimerOpts, peer: Peer) -> Self {
        let (tx, rx) = mpsc::channel::<SessionEvent>();
        Self {
            peer,
            state: BGPState::Idle,
            timers: Timers::new(timer_cfg),
            tx_event_chan: tx,
            rx_event_chan: rx,
            opts: cfg,
        }
    }

    pub fn apply_fsm_event(&mut self, event: BGPEvent) -> Result<(), String> {
        let next_state = fsm::on_event(self.state, event)?;
        println!(
            "[FSM] <peer={}> {:?} -> {:?}",
            self.peer.get_ip(),
            self.state,
            next_state
        );
        self.state = next_state;
        Ok(())
    }

    pub fn is_established(&self) -> bool {
        self.state == BGPState::Established
    }

    pub fn initiate(&mut self) -> Result<(), String> {
        let open_msg = OpenMessage {
            version: 4,
            asn: self.opts.local_as,
            hold_time: self.timers.cfg.hold_interval.as_secs() as u16,
            bgp_id: util::ipv4_str_to_u32(&self.opts.router_id)?,
            opt_len: 0,
            opts: Vec::new(),
        };
        println!("[SENDER] Sending OPEN: {:?}", open_msg);
        let bgp_msg = BGPMessage::Open(open_msg);

        self.peer.send_message(bgp_msg).map_err(|e| e.to_string())?;
        self.apply_fsm_event(BGPEvent::LocalStart)?;

        Ok(())
    }

    pub fn teardown(&mut self) -> Result<(), String> {
        println!("[SESSION] Tearing down connection with peer");
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
                println!("[SESSION] Terminating session: {}", e);
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

    fn handle_msg(&mut self, msg: BGPMessage) -> Result<(), String> {
        match msg {
            BGPMessage::Open(open) => {
                println!("[HANDLE_MSG] Got OPEN: {:?}", open);
                // For OPEN:
                // - Transition to OpenConfirm
                // - Configure hold timer from msg
                // - Send KeepAlive
                self.apply_fsm_event(BGPEvent::OpenReceived)?;
                let hold_interval = Duration::from_secs(open.hold_time as u64);
                self.timers.set_hold_interval(hold_interval);
                println!("[SENDER] Sending KEEPALIVE");
                self.peer.send_message(BGPMessage::KeepAlive)
            }
            BGPMessage::KeepAlive => {
                println!("[HANDLE_MSG] Got KEEPALIVE");
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
                println!("[HANDLE_MSG] Got NOTIFICATION: {:?}", notification);
                // For NOTIFICATION:
                // - Check error code
                // - Most cases require session teardown
                self.teardown()?;
                Err(format!("Received notification {:?}", notification.err_code))
            }
        }
    }

    pub fn handle_keepalive_expiry(&mut self) -> Result<(), String> {
        println!("[KEEPALIVE] Timer expired, sending new KEEPALIVE message");
        self.peer.send_message(BGPMessage::KeepAlive)?;
        self.timers.update_last_keepalive_tx();
        Ok(())
    }

    pub fn handle_hold_expiry(&mut self) -> Result<(), String> {
        println!("[HANDLE_MSG] Timer expired, sending notification message");
        let notification_msg =
            NotificationMessage::new(NotificationErrorCode::HoldTimerExpired, 0, Vec::new());
        self.peer
            .send_message(BGPMessage::Notification(notification_msg))?;
        println!("[HOLD] Timer expired, tearing down session");
        self.teardown()?;
        Err("Hold timer expired, terminating session".to_string())
    }

    pub fn handle_hold_refresh(&mut self) -> Result<(), String> {
        println!("[HOLD] Got keepalive, refreshing hold timer");
        self.timers.update_last_keepalive_rx();
        Ok(())
    }

    pub fn handle_peer_disconnect(&mut self) -> Result<(), String> {
        println!("[SESSION] Peer disconnected");
        self.teardown()?;
        Err("Peer disconnected, terminating session".to_string())
    }

    pub fn start_reader_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        println!("[THREAD SPAWN] Start reader thread");
        let mut peer_reader = self.peer.clone_reader().unwrap();
        std::thread::spawn(move || {
            loop {
                match peer_reader.recv_message() {
                    Ok(msg) => {
                        println!("[READER] Got message");
                        if tx_event_chan
                            .send(SessionEvent::MessageReceived(msg))
                            .is_err()
                        {
                            println!("[READER] Session already gone");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("[READER] {}", e);
                        if tx_event_chan
                            .send(SessionEvent::PeerDisconnected())
                            .is_err()
                        {
                            println!("[READER] Session already gone");
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

        if self.timers.cfg.enable_timer_monitors {
            // - KEEPALIVE monitor: To dump value of keepalive timer
            // - HOLD monitor: To dump value of hold timer
            self.timers.start_hold_monitor();
            self.timers.start_keepalive_monitor();
        }
    }
}
