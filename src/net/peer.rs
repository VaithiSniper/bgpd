use crate::fsm::BGPState;
use crate::packet::{parse_message, BGPMessage};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};

pub struct Peer {
    pub stream: TcpStream,
    state: BGPState,
    socket_addr: SocketAddr,
    ip_addr: IpAddr,
}

impl Peer {
    pub fn new(stream: TcpStream, socket_addr: SocketAddr) -> Peer {
        Peer {
            state: BGPState::Idle,
            stream,
            socket_addr,
            ip_addr: socket_addr.ip(),
        }
    }
    pub fn transition(&mut self, new_state: BGPState) {
        println!(
            "Transitioning BGP State for peer={} from {:?} to {:?}",
            self.ip_addr, self.state, new_state
        );
        self.state = new_state;
    }
    pub fn send_message(&mut self, bgp_msg: BGPMessage) -> Result<(), String> {
        let bytes = bgp_msg.serialize();
        self.stream.write_all(&bytes).map_err(|e| e.to_string())?;

        Ok(())
    }
    pub fn recv_message(&mut self) -> Result<BGPMessage, String> {
        let mut buf = [0; 4096];

        let n_bytes = self.stream.read(&mut buf).map_err(|e| e.to_string())?;

        if n_bytes == 0 {
            return Err("Peer disconnected".to_string());
        }

        parse_message(&buf[..n_bytes])
    }
}
