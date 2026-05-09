mod cli;
mod compare;
mod config;
mod network;
mod profiles;
mod report;
mod windows;

use std::env;

pub const VERSION: &str = "v2.0.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    let effective_config = config::load_config();

    cli::handle_args(&args, effective_config);
}
