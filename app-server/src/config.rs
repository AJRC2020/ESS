use serde::Deserialize;

use server_common::{server_config, AuthClientConfig};

server_config! {
    "app-server",
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Config {
        pub auth_server: AuthClientConfig,
        pub fileshare_server: ServiceClientConfig,
        pub filestore_server: ServiceClientConfig,
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceClientConfig {
    host: String,
    port: u16,
}

impl ServiceClientConfig {
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
