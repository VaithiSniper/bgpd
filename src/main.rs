mod bgp;
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
        RunningMode::Client => net::start_client(&cli_args.address),
        RunningMode::Both => {
            println!("Running as both server and client")
        }
    }
}
