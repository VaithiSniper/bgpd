use crate::packet::{parse_message, BGPMessage};
use std::io::Read;
use std::net::TcpListener;

pub struct ServerOpts {
    pub listen_addr: String,
    pub listen_port: u16,
    pub full_listen_addr: String,
}
impl ServerOpts {
    fn get_full_listen_addr(self) -> String {
        let port_str = self.listen_port.to_string();
        self.listen_addr + ":" + &port_str
    }
}
pub fn start_server(server_opts: ServerOpts) {
    let listener = TcpListener::bind(&server_opts.full_listen_addr).unwrap();
    println!("Listening on {}", server_opts.full_listen_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("Peer connected: {}", stream.peer_addr().unwrap());
                let mut buf = [0; 4096];

                let n_read = stream.read(&mut buf).unwrap();
                println!("Read {} bytes", n_read);

                let bgp_message = parse_message(&buf).unwrap();
                match bgp_message {
                    BGPMessage::Open(open) => {
                        println!("Got OPEN message with values {:?}", open);
                    }
                }
            }
            Err(e) => {
                println!("Error while connecting: {}", e);
            }
        }
    }
}
