use crate::fsm::BGPState;
use crate::fsm::BGPState::{Established, OpenConfirm, OpenSent};
use crate::net::Peer;
use crate::packet::{BGPMessage, OpenMessage};
use crate::util;
use std::any::Any;
use std::net::Shutdown;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

const HOLD_INTERVAL_S: u64 = 90;
const KEEPALIVE_INTERVAL_S: u64 = HOLD_INTERVAL_S / 3;

pub enum SessionEvent {
    MessageReceived(BGPMessage),
    HoldTimerRefresh,
    HoldTimerExpired,
    KeepAliveTimerExpired,
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

    pub fn teardown(&mut self) -> Result<BGPState, String> {
        println!("Tearing down connection with peer");
        self.peer.stream.shutdown(Shutdown::Both).unwrap();
        self.peer.transition(BGPState::Idle)
    }

    pub fn run(&mut self) {
        // Start reader/writer threads and go into event loop
        let tx_clone_reader = self.tx_event_chan.clone();
        self.start_reader_thread(tx_clone_reader);
        loop {
            let event = self.rx_event_chan.recv().unwrap();
            self.dispatch_event_handler(event);
        }
    }

    pub fn dispatch_event_handler(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::MessageReceived(msg) => self.handle_msg(msg),
            SessionEvent::KeepAliveTimerExpired => self.handle_keepalive_expiry(),
            SessionEvent::HoldTimerExpired => self.handle_hold_expiry(),
            SessionEvent::HoldTimerRefresh => self.handle_hold_refresh(),
        }
    }

    fn handle_msg(&mut self, msg: BGPMessage) {
        match msg {
            BGPMessage::Open(open) => {
                println!("[HANDLE_MSG] Got OPEN: {:?}", open);
                // For OPEN:
                // - Transition to OpenConfirm
                // - Send KeepAlive
                self.peer.transition(OpenConfirm).unwrap();
                let keepalive = BGPMessage::KeepAlive;
                println!("Sending KEEPALIVE");
                self.peer.send_message(keepalive).unwrap();
            }
            BGPMessage::KeepAlive => {
                println!("[HANDLE_MSG] Got KEEPALIVE");
                // For KEEPALIVE:
                // - Transition to Established
                // - If first time transitioning into establishing:
                //     - Start timers
                // - Refresh hold timer
                let was_established = self.peer.is_established();
                self.peer.transition(Established).unwrap();
                if !was_established {
                    // If it is freshly established, setup timers in threads
                    self.start_timer_threads();
                }
                self.tx_event_chan
                    .send(SessionEvent::HoldTimerRefresh)
                    .unwrap();
            }
        }
    }

    pub fn handle_keepalive_expiry(&mut self) {
        println!("[KEEPALIVE] Timer expired, sending new KEEPALIVE message");
        self.peer.send_message(BGPMessage::KeepAlive).unwrap();
        self.timers.update_last_keepalive_tx();
    }

    pub fn handle_hold_expiry(&mut self) {
        println!("[HOLD] Timer expired, tearing down session");
    }

    pub fn handle_hold_refresh(&mut self) {
        println!("[HOLD] Got keepalive, refreshing hold timer");
        self.timers.update_last_keepalive_rx();
    }

    pub fn start_reader_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        println!("[THREAD SPAWN] Start reader thread");
        let mut peer_reader = self.peer.clone_reader().unwrap();
        std::thread::spawn(move || {
            loop {
                let msg = peer_reader.recv_message().unwrap();
                println!("[READER] Got message {:?}", msg.type_id());
                tx_event_chan
                    .send(SessionEvent::MessageReceived(msg))
                    .unwrap();
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
    pub keepalive_interval: Duration,
    pub hold_interval: Duration,
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
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > KEEPALIVE_INTERVAL_S {
                    println!("Keepalive timer expired");
                    tx_event_chan
                        .send(SessionEvent::KeepAliveTimerExpired)
                        .unwrap();
                }
            }
        });
    }

    pub fn start_hold_timer_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        println!("[THREAD_SPAWN] Start hold timer thread");
        let timer = Arc::clone(&self.last_keepalive_rx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > HOLD_INTERVAL_S {
                    println!("Hold timer expired");
                    tx_event_chan.send(SessionEvent::HoldTimerExpired).unwrap();
                }
            }
        });
    }

    pub fn start_keepalive_monitor(&self) {
        let timer = Arc::clone(&self.last_keepalive_tx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(HOLD_INTERVAL_S / 3));
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
                std::thread::sleep(std::time::Duration::from_secs(HOLD_INTERVAL_S / 9));
                let timer_inst = timer.lock().unwrap();
                println!(
                    "Hold timer elapsed seconds -- {}",
                    timer_inst.elapsed().as_secs()
                );
            }
        });
    }
}
