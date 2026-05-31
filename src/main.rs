mod bgp;
mod config;
mod environment;
mod fsm;
mod net;
mod packet;
mod util;

use crate::environment::get_args;
use crate::net::{Router, RouterOpts};

fn main() {
    let cli_args = get_args();
    let opts = RouterOpts::new(cli_args.config_file_path).unwrap();
    let mut router = Router::new(opts).unwrap();

    router.start();
}
