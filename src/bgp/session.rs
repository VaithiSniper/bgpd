use crate::fsm::BGPState;
use crate::fsm::BGPState::{Established, OpenConfirm, OpenSent};
use crate::net::Peer;
use crate::packet::{BGPMessage, OpenMessage};
use crate::util;
use std::net::Shutdown;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

pub const HOLD_INTERVAL_S: u64 = 10;
pub const KEEPALIVE_INTERVAL_S: u64 = HOLD_INTERVAL_S * 9 / 3;

pub enum SessionEvent {
    MessageReceived(BGPMessage),
    HoldTimerRefresh,
    HoldTimerExpired,
    KeepAliveTimerExpired,
    PeerDisconnected(),
}

pub struct Session {
    pub peer: Peer,
    pub timers: Timers,
    pub tx_event_chan: mpsc::Sender<SessionEvent>,
    pub rx_event_chan: mpsc::Receiver<SessionEvent>,
}

impl Session {
    pub fn new(peer: Peer) -> Self {
        let (tx, rx) = mpsc::channel::<SessionEvent>();
        Self {
            peer,
            timers: Timers::new(false),
            tx_event_chan: tx,
            rx_event_chan: rx,
        }
    }

    pub fn initiate(&mut self) -> Result<(), String> {
        let bgp_id =
            util::ipv4_str_to_u32("192.168.0.108").map_err(|_| "Invalid IP address".to_string())?;
        let open_msg = OpenMessage {
            version: 4,
            asn: 65001,
            hold_time: HOLD_INTERVAL_S as u16,
            bgp_id,
            opt_len: 0,
            opts: Vec::new(),
        };
        println!("[SENDER] Sending OPEN: {:?}", open_msg);
        let bgp_msg = BGPMessage::Open(open_msg);

        self.peer.send_message(bgp_msg).map_err(|e| e.to_string())?;
        self.peer.transition(OpenSent)?;

        Ok(())
    }

    pub fn teardown(&mut self) -> Result<(), String> {
        println!("[SESSION] Tearing down connection with peer");
        self.peer.stream.shutdown(Shutdown::Both).unwrap();
        self.peer.transition(BGPState::Idle)?;

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
                self.peer.transition(OpenConfirm)?;
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
                let was_established = self.peer.is_established();
                self.peer.transition(Established)?;
                if !was_established {
                    // If it is freshly established, setup timers in threads
                    self.start_timer_threads();
                }
                self.tx_event_chan
                    .send(SessionEvent::HoldTimerRefresh)
                    .map_err(|e| e.to_string())
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

        if self.timers.enable_timer_monitors {
            // - KEEPALIVE monitor: To dump value of keepalive timer
            // - HOLD monitor: To dump value of hold timer
            self.timers.start_hold_monitor();
            self.timers.start_keepalive_monitor();
        }
    }
}

pub struct Timers {
    last_keepalive_tx: Arc<Mutex<Instant>>, // Last keepalive sent (For keepalive timer)
    last_keepalive_rx: Arc<Mutex<Instant>>, // Last keepalive recv (For hold timer)
    keepalive_interval: Duration,
    hold_interval: Duration,
    pub enable_timer_monitors: bool,
}

impl Timers {
    pub fn new(enable_timer_monitors: bool) -> Self {
        Self {
            last_keepalive_tx: Arc::new(Mutex::new(Instant::now())),
            last_keepalive_rx: Arc::new(Mutex::new(Instant::now())),
            keepalive_interval: Duration::from_secs(KEEPALIVE_INTERVAL_S),
            hold_interval: Duration::from_secs(HOLD_INTERVAL_S),
            enable_timer_monitors,
        }
    }

    pub fn set_hold_interval(&mut self, hold_interval: Duration) {
        self.hold_interval = hold_interval;
    }

    pub fn update_last_keepalive_tx(&mut self) {
        let mut timer = self.last_keepalive_tx.lock().unwrap();
        *timer = Instant::now();
    }

    pub fn update_last_keepalive_rx(&mut self) {
        let mut timer = self.last_keepalive_rx.lock().unwrap();
        *timer = Instant::now();
    }

    pub fn start_keepalive_timer_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        println!("[THREAD_SPAWN] Start keepalive timer thread");
        let timer = Arc::clone(&self.last_keepalive_tx);
        let interval = self.keepalive_interval;
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > interval.as_secs() {
                    if tx_event_chan
                        .send(SessionEvent::KeepAliveTimerExpired)
                        .is_err()
                    {
                        println!("[KEEPALIVE_THREAD] Session already gone");
                        break;
                    }
                }
            }
        });
    }

    pub fn start_hold_timer_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        println!("[THREAD_SPAWN] Start hold timer thread");
        let timer = Arc::clone(&self.last_keepalive_rx);
        let interval = self.hold_interval;
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > interval.as_secs() {
                    if tx_event_chan.send(SessionEvent::HoldTimerExpired).is_err() {
                        println!("[HOLD_THREAD] Session already gone");
                        break;
                    }
                }
            }
        });
    }

    pub fn start_keepalive_monitor(&self) {
        let timer = Arc::clone(&self.last_keepalive_tx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(HOLD_INTERVAL_S / 3));
                let timer_inst = timer.lock().unwrap();
                println!(
                    "Hold timer elapsed seconds -- {}",
                    timer_inst.elapsed().as_secs()
                );
            }
        });
    }

    pub fn start_hold_monitor(&self) {
        let timer = Arc::clone(&self.last_keepalive_rx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(HOLD_INTERVAL_S / 9));
                let timer_inst = timer.lock().unwrap();
                println!(
                    "Hold timer elapsed seconds -- {}",
                    timer_inst.elapsed().as_secs()
                );
            }
        });
    }
}
