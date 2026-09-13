use crate::bgp::session::SessionEvent;
use crate::util::{log_debug, log_error};
use std::cmp::min;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

const MODULE: &str = "bgp.session";

pub const MONITOR_PRINT_INTERVAL_KEEPALIVE_S: u64 = 10;
pub const MONITOR_PRINT_INTERVAL_HOLD_S: u64 = MONITOR_PRINT_INTERVAL_KEEPALIVE_S * 3;

#[derive(Debug, Clone)]
pub struct TimerConfig {
    pub keepalive_interval: Duration,
    pub hold_interval: Duration,
}
impl TimerConfig {
    pub fn new(keepalive_interval: u64, hold_interval: u64) -> TimerConfig {
        let keepalive_interval = Duration::from_secs(keepalive_interval);
        let hold_interval = Duration::from_secs(hold_interval);
        TimerConfig {
            keepalive_interval,
            hold_interval,
        }
    }
}

pub struct Timers {
    last_keepalive_tx: Arc<Mutex<Instant>>, // Last keepalive sent (For keepalive timer)
    last_keepalive_rx: Arc<Mutex<Instant>>, // Last keepalive recv (For hold timer)
    pub local_cfg: TimerConfig,
    pub negotiated_cfg: TimerConfig,
}

impl Timers {
    pub fn new(cfg: TimerConfig) -> Self {
        Self {
            last_keepalive_tx: Arc::new(Mutex::new(Instant::now())),
            last_keepalive_rx: Arc::new(Mutex::new(Instant::now())),
            local_cfg: cfg.clone(),
            negotiated_cfg: cfg,
        }
    }

    pub fn negotiate(&mut self, peer_hold_time: u16) {
        let local_hold_interval = self.local_cfg.hold_interval;
        let peer_hold_interval = Duration::from_secs(peer_hold_time as u64);
        // Per RFC, pick min
        let neg_hold_interval = min(local_hold_interval, peer_hold_interval);
        let neg_keepalive_interval = neg_hold_interval / 3;
        // Update self
        self.negotiated_cfg.hold_interval = neg_hold_interval;
        self.negotiated_cfg.keepalive_interval = neg_keepalive_interval;
        log_debug(
            MODULE,
            "Negotiated Timers",
            &[
                (
                    "local_hold_interval",
                    local_hold_interval.as_secs().to_string(),
                ),
                (
                    "peer_hold_interval",
                    peer_hold_interval.as_secs().to_string(),
                ),
                ("neg_hold_interval", neg_hold_interval.as_secs().to_string()),
                (
                    "neg_keepalive_interval",
                    neg_keepalive_interval.as_secs().to_string(),
                ),
            ],
        );
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
        log_debug(
            MODULE,
            "Starting keepalive timer thread",
            &[(
                "interval",
                format!("{}", self.negotiated_cfg.hold_interval.as_secs()),
            )],
        );
        let timer = Arc::clone(&self.last_keepalive_tx);
        let interval = self.negotiated_cfg.keepalive_interval;
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > interval.as_secs() {
                    if tx_event_chan
                        .send(SessionEvent::KeepAliveTimerExpired)
                        .is_err()
                    {
                        log_error(MODULE, "Session already gone", &[]);
                        break;
                    }
                }
            }
        });
    }

    pub fn start_hold_timer_thread(&mut self, tx_event_chan: mpsc::Sender<SessionEvent>) {
        log_debug(
            MODULE,
            "Starting hold timer thread",
            &[(
                "interval",
                format!("{}", self.negotiated_cfg.hold_interval.as_secs()),
            )],
        );
        let timer = Arc::clone(&self.last_keepalive_rx);
        let interval = self.negotiated_cfg.hold_interval;
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > interval.as_secs() {
                    if tx_event_chan.send(SessionEvent::HoldTimerExpired).is_err() {
                        log_error(MODULE, "Session already gone", &[]);
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
                std::thread::sleep(Duration::from_secs(MONITOR_PRINT_INTERVAL_KEEPALIVE_S));
                let timer_inst = timer.lock().unwrap();
                log_debug(
                    MODULE,
                    "Keepalive timer elapsed seconds",
                    &[("seconds", timer_inst.elapsed().as_secs().to_string())],
                );
            }
        });
    }

    pub fn start_hold_monitor(&self) {
        let timer = Arc::clone(&self.last_keepalive_rx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(MONITOR_PRINT_INTERVAL_HOLD_S));
                let timer_inst = timer.lock().unwrap();
                log_debug(
                    MODULE,
                    "Hold timer elapsed seconds",
                    &[("seconds", timer_inst.elapsed().as_secs().to_string())],
                );
            }
        });
    }
}
