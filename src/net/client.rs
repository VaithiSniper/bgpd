use std::io::Write;
use std::net::TcpStream;

pub fn start_client_and_send_msg(peer_addr: String, data: &[u8]) {
    let mut stream = TcpStream::connect(&peer_addr).unwrap();
    println!("Connected to server: {}", peer_addr);

    stream.write_all(data).unwrap();

    println!("Wrote {} bytes", data.len());
}
