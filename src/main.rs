mod environment;
mod net;
mod packet;

use crate::environment::{get_args, RunningMode};
use crate::net::{start_client_and_send_msg, start_server, ServerOpts};

fn main() {
    let cli_args = get_args();

    match cli_args.running_mode {
        RunningMode::Server => {
            let opts = ServerOpts {
                listen_addr: "".to_string(),
                listen_port: 0,
                full_listen_addr: cli_args.address,
            };
            start_server(opts);
        }
        RunningMode::Client => {
            let open_msg = packet::OpenMessage {
                version: 4,
                asn: 65001,
                hold_time: 90,
                bgp_id: 0x01010101,
            };
            let bytes = open_msg.serialize();
            start_client_and_send_msg(cli_args.address, bytes.as_slice());
        }
        RunningMode::Both => {
            println!("Running as both server and client")
        }
    }
}
