use crate::bgp::session::Session;
use crate::net::peer::Peer;
use std::net::TcpListener;

pub struct ServerOpts {
    pub full_listen_addr: String,
}

pub fn start_server(server_opts: ServerOpts) {
    println!("Started server");
    let listener = TcpListener::bind(&server_opts.full_listen_addr).unwrap();
    println!("Listening on {}", server_opts.full_listen_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_socket_addr = stream.peer_addr().unwrap();
                println!("Peer connected: {}", peer_socket_addr);
                let peer: Peer = Peer::new(stream, peer_socket_addr);
                let mut session: Session = Session::new(peer);
                session.run();
            }
            Err(e) => {
                println!("Error while accepting: {}", e);
            }
        }
    }
}
