use crate::fsm::BGPState::{Established, OpenConfirm, OpenSent};
use crate::net::Peer;
use crate::packet::{BGPMessage, OpenMessage};
use crate::util;
use std::sync::{Arc, Mutex};

pub struct Session {
    pub peer: Arc<Mutex<Peer>>,
}

impl Session {
    pub fn new(peer: Peer) -> Self {
        Self {
            peer: Arc::new(Mutex::new(peer)),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let msg = {
                let mut peer = self.peer.lock().unwrap();
                peer.recv_message()?
            };
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
                let mut peer = self.peer.lock().unwrap();
                peer.transition(OpenConfirm)?;
                let keepalive = BGPMessage::KeepAlive;
                println!("Sending KEEPALIVE");
                peer.send_message(keepalive)
            }
            BGPMessage::KeepAlive => {
                println!("Got KEEPALIVE");
                // For KEEPALIVE:
                // - Transition to Established
                // - Start self KEEPALIVE timer
                let mut peer = self.peer.lock().unwrap();
                let was_established = peer.is_established();
                peer.transition(Established)?;
                drop(peer);
                if !was_established {
                    // If it is freshly established, setup timer
                    self.start_keepalive_timer()?;
                }
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

        let mut peer = self.peer.lock().unwrap();
        peer.send_message(bgp_msg).map_err(|e| e.to_string())?;
        peer.transition(OpenSent)?;

        Ok(())
    }

    pub fn start_keepalive_timer(&self) -> Result<(), String> {
        println!("Start keepalive timer");
        let peer = Arc::clone(&self.peer);
        std::thread::spawn(move || {
            loop {
                println!("Waiting for keepalive timer interval of 5s");
                std::thread::sleep(std::time::Duration::from_secs(5));

                println!("Sending periodic KEEPALIVE");
                let mut peer = peer.lock().unwrap();
                if let Err(e) = peer.send_message(BGPMessage::KeepAlive) {
                    println!("send_message err: {:?}", e);
                    break;
                }
                drop(peer);
            }
        });
        Ok(())
    }
}
