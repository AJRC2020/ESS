use crate::config::Config;
use server_common::util::new_reqwest_client_from_certificates;

#[derive(Clone, Debug)]
pub struct State {
    pub client: reqwest::Client,
    pub config: Config,
}

pub type AppState = State;

pub fn get_state(config: Config) -> anyhow::Result<AppState> {
    Ok(State {
        client: new_reqwest_client_from_certificates("app-server")?,
        config,
    })
}
