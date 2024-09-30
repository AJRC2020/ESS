mod config;
mod server;
mod state;
mod user;

use server_common::prelude::*;

use crate::config::Config;
use crate::server::get_router;
use crate::state::{get_state, AppState};

server_args!("cfg/auth-server.toml");

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    server_common::server_main::<Config, AppState>(&args, get_router(), get_state)
}
