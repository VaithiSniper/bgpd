mod environment;
mod fsm;
mod net;
mod packet;
mod util;

use crate::environment::{get_args, RunningMode};

fn main() {
    let cli_args = get_args();

    match cli_args.running_mode {
        RunningMode::Server => {
            let opts = net::ServerOpts {
                listen_addr: "".to_string(),
                listen_port: 0,
                full_listen_addr: cli_args.address,
            };
            net::start_server(opts);
        }
        RunningMode::Client => {
            let open_msg = packet::OpenMessage {
                version: 4,
                asn: 65001,
                hold_time: 90,
                bgp_id: util::ipv4_str_to_u32("192.168.0.108").unwrap(),
                opt_len: 0,
                opts: Vec::new(),
            };
            let bgp_msg = packet::BGPMessage::Open(open_msg);
            let bytes = bgp_msg.serialize();
            net::start_client_and_send_msg(cli_args.address, bytes.as_slice());
        }
        RunningMode::Both => {
            println!("Running as both server and client")
        }
    }
}
