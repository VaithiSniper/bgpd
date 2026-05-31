use crate::packet::{parse_message, BGPMessage};
use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};

pub struct Peer {
    pub stream: TcpStream,
    socket_addr: SocketAddr,
}

impl Peer {
    pub fn new(stream: TcpStream, socket_addr: SocketAddr) -> Peer {
        Peer {
            stream,
            socket_addr,
        }
    }

    pub fn close(&mut self) -> Result<(), String> {
        self.stream
            .shutdown(Shutdown::Both)
            .map_err(|err| err.to_string())
    }

    pub fn get_ip(&self) -> IpAddr {
        self.socket_addr.ip()
    }

    pub fn clone_reader(&self) -> Result<PeerReader, String> {
        let cloned_stream = self.stream.try_clone().map_err(|e| e.to_string())?;
        Ok(PeerReader {
            stream: cloned_stream,
        })
    }

    pub fn send_message(&mut self, bgp_msg: BGPMessage) -> Result<(), String> {
        let bytes = bgp_msg.serialize();
        self.stream.write_all(&bytes).map_err(|e| e.to_string())?;

        Ok(())
    }
}

pub struct PeerReader {
    stream: TcpStream,
}

impl PeerReader {
    pub fn recv_message(&mut self) -> Result<BGPMessage, String> {
        let mut buf = [0; 4096];

        let n_bytes = self.stream.read(&mut buf).map_err(|e| e.to_string())?;

        if n_bytes == 0 {
            return Err("Peer disconnected".to_string());
        }

        parse_message(&buf[..n_bytes])
    }
}
