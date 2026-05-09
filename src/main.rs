mod cli;
mod compare;
mod config;
mod doctor_system;
mod firewall;
mod groups;
mod logger;
mod network;
mod profiles;
mod report;
mod services;
mod windows;

use std::env;

pub const VERSION: &str = "v2.2.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    let effective_config = config::load_config();

    cli::handle_args(&args, effective_config);
}
