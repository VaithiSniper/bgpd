use crate::fsm::BGPState::{Established, OpenConfirm, OpenSent};
use crate::net::Peer;
use crate::packet::{BGPMessage, OpenMessage};
use crate::util;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const HOLD_TIMER_S: u64 = 90;
const KEEPALIVE_INTERVAL_S: u64 = HOLD_TIMER_S / 3;

pub struct Session {
    pub peer: Peer,
    pub keepalive_last_rx: Arc<Mutex<Instant>>,
}

impl Session {
    pub fn new(peer: Peer) -> Self {
        Self {
            peer,
            keepalive_last_rx: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let msg = { self.peer.recv_message()? };
            if let Err(e) = self.handle_msg(msg) {
                println!("handle_msg err: {:?}", e);
                return Err(e);
            }
        }
    }

    fn handle_msg(&mut self, msg: BGPMessage) -> Result<(), String> {
        match msg {
            BGPMessage::Open(open) => {
                println!("Got OPEN: {:?}", open);
                // For OPEN:
                // - Transition to OpenConfirm
                // - Send KeepAlive
                self.peer.transition(OpenConfirm)?;
                let keepalive = BGPMessage::KeepAlive;
                println!("Sending KEEPALIVE");
                self.peer.send_message(keepalive)
            }
            BGPMessage::KeepAlive => {
                println!("Got KEEPALIVE");
                // For KEEPALIVE:
                // - Transition to Established
                // - Start self KEEPALIVE timer
                // - Start HOLD timer
                let was_established = self.peer.is_established();
                self.peer.transition(Established)?;
                if !was_established {
                    // If it is freshly established, setup timers
                    self.start_keepalive_timer()?;
                    self.enforce_hold_timer()?;
                    self.dump_hold_timer()?;
                }
                self.update_hold_timer()?;
                Ok(())
            }
        }
    }

    pub fn initiate(&mut self) -> Result<(), String> {
        let bgp_id =
            util::ipv4_str_to_u32("192.168.0.108").map_err(|_| "Invalid IP address".to_string())?;
        let open_msg = OpenMessage {
            version: 4,
            asn: 65001,
            hold_time: 90,
            bgp_id,
            opt_len: 0,
            opts: Vec::new(),
        };
        println!("Sending OPEN: {:?}", open_msg);
        let bgp_msg = BGPMessage::Open(open_msg);

        self.peer.send_message(bgp_msg).map_err(|e| e.to_string())?;
        self.peer.transition(OpenSent)?;

        Ok(())
    }

    pub fn start_keepalive_timer(&mut self) -> Result<(), String> {
        println!("Start keepalive timer");
        let mut cloned_stream = self.peer.clone_stream()?;

        std::thread::spawn(move || {
            loop {
                println!("Next KEEPALIVE after interval of {}s", KEEPALIVE_INTERVAL_S);
                std::thread::sleep(std::time::Duration::from_secs(KEEPALIVE_INTERVAL_S));

                println!("Sending periodic KEEPALIVE");
                let keepalive_msg_bytes = BGPMessage::KeepAlive.serialize();
                if let Err(e) = cloned_stream.write_all(&keepalive_msg_bytes) {
                    println!("Error while writing keepalive to stream, err: {:?}", e);
                    break;
                }
            }
        });
        Ok(())
    }

    pub fn update_hold_timer(&mut self) -> Result<(), String> {
        println!("Refreshing hold timer since we got KEEPALIVE");
        let mut timer = self.keepalive_last_rx.lock().unwrap();
        *timer = Instant::now();
        drop(timer);

        Ok(())
    }

    pub fn enforce_hold_timer(&mut self) -> Result<(), String> {
        let timer = Arc::clone(&self.keepalive_last_rx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let timer_inst = timer.lock().unwrap();
                if timer_inst.elapsed().as_secs() > KEEPALIVE_INTERVAL_S / 10 {
                    println!("Hold timer expired!");
                    // Teardown session
                    break;
                }
            }
        });
        Ok(())
    }

    pub fn dump_hold_timer(&mut self) -> Result<(), String> {
        let timer = Arc::clone(&self.keepalive_last_rx);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(HOLD_TIMER_S / 9));
                let timer_inst = timer.lock().unwrap();
                println!(
                    "Hold timer elapsed seconds -- {}",
                    timer_inst.elapsed().as_secs()
                );
            }
        });
        Ok(())
    }
}
