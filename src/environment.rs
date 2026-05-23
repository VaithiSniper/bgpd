use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum RunningMode {
    Server,
    Client,
    Both,
}
#[derive(Debug, Parser)]
#[command(name = "bgpd")]
#[command(version = "1.0")]
#[command(about = "A minimal BGP daemon")]
#[command(
    long_about = "A minimal BGP daemon written in Rust. Uses TOML config files and supports BGP unicast and Unnumbered"
)]
pub struct CLIArgs {
    #[arg(value_enum)]
    pub running_mode: RunningMode,
    pub address: String,
}

pub fn get_args() -> CLIArgs {
    CLIArgs::parse()
}
