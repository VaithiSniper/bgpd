use std::io::Read;
use std::net::TcpListener;
use crate::packet::parse_header;

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

                match parse_header(&buf[0..n_read]) {
                    Ok(header) => {
                        println!("Header successfully parsed: {:?}", header);
                    }
                    Err(e) => {
                        println!("Error parsing header: {:?}", e);
                    }
                }

                println!("Read {} bytes", n_read);
            }
            Err(e) => {
                println!("Error while connecting: {}", e);
            }
        }
    }
}