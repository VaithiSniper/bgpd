use crate::bgp::session::SessionEvent;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

pub const MONITOR_PRINT_INTERVAL_KEEPALIVE_S: u64 = 10;
pub const MONITOR_PRINT_INTERVAL_HOLD_S: u64 = MONITOR_PRINT_INTERVAL_KEEPALIVE_S * 3;

#[derive(Debug, Clone)]
pub struct TimerOpts {
    keepalive_interval: Duration,
    pub hold_interval: Duration,
    pub enable_timer_monitors: bool,
}
impl TimerOpts {
    pub fn new(
        keepalive_interval: u64,
        hold_interval: u64,
        enable_timer_monitors: bool,
    ) -> TimerOpts {
        let keepalive_interval = Duration::from_secs(keepalive_interval);
        let hold_interval = Duration::from_secs(hold_interval);
        TimerOpts {
            keepalive_interval,
            hold_interval,
            enable_timer_monitors,
        }
    }
}

pub struct Timers {
    last_keepalive_tx: Arc<Mutex<Instant>>, // Last keepalive sent (For keepalive timer)
    last_keepalive_rx: Arc<Mutex<Instant>>, // Last keepalive recv (For hold timer)
    pub cfg: TimerOpts,
}

impl Timers {
    pub fn new(cfg: TimerOpts) -> Self {
        Self {
            last_keepalive_tx: Arc::new(Mutex::new(Instant::now())),
            last_keepalive_rx: Arc::new(Mutex::new(Instant::now())),
            cfg,
        }
    }

    pub fn set_hold_interval(&mut self, hold_interval: Duration) {
        self.cfg.hold_interval = hold_interval;
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
        let interval = self.cfg.keepalive_interval;
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
        let interval = self.cfg.hold_interval;
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
                std::thread::sleep(Duration::from_secs(MONITOR_PRINT_INTERVAL_KEEPALIVE_S));
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
                std::thread::sleep(Duration::from_secs(MONITOR_PRINT_INTERVAL_HOLD_S));
                let timer_inst = timer.lock().unwrap();
                println!(
                    "Hold timer elapsed seconds -- {}",
                    timer_inst.elapsed().as_secs()
                );
            }
        });
    }
}
