use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Listen to CAN frames and print them out to the console
    Listen,

    /// Send Ping-frame and expect Pong
    Ping {
        /// Period in seconds between pings
        #[arg(short, long, value_name = "SECONDS", default_value = "5")]
        period: u64,
    },

    /// Test throughput by sending ping messages as fast as possible
    Throughput,
}
