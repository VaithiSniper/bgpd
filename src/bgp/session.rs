use crate::fsm::BGPState::{Established, OpenConfirm};
use crate::net::Peer;
use crate::packet::{BGPMessage, OpenMessage};
use crate::util;

pub struct Session {
    pub peer: Peer,
}

impl Session {
    pub fn new(peer: Peer) -> Self {
        Self { peer }
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let msg = self.peer.recv_message()?;
            self.handle_msg(msg)?
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
                self.peer.transition(Established)?;

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

        Ok(())
    }
}
