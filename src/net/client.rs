use crate::bgp::session::Session;
use crate::net::peer::Peer;
use std::net::TcpStream;

pub fn start_client(peer_addr: &str) {
    println!("Started client");
    let mut session = create_session_with_peer(peer_addr).unwrap();
    session.initiate().unwrap();
    session.run();
}

pub fn create_session_with_peer(peer_addr: &str) -> Result<Session, String> {
    let stream = TcpStream::connect(peer_addr).map_err(|e| e.to_string())?;
    let peer_socket_addr = stream.peer_addr().map_err(|e| e.to_string())?;
    println!("Connected to peer: {}", peer_socket_addr);
    Ok(Session::new(Peer::new(stream, peer_socket_addr)))
}
