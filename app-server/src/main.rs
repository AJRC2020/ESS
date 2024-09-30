mod config;
mod server;
mod state;

use server_common::prelude::*;
use state::{get_state, AppState};

use crate::config::Config;
use crate::server::get_router;

server_args!("cfg/app-server.toml");

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    server_common::server_main::<Config, AppState>(&args, get_router(), get_state)
}
