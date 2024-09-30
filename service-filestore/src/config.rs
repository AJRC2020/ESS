use std::collections::HashSet;
use std::net::IpAddr;

use serde::Deserialize;

use server_common::user::Role;
use server_common::{server_config, AuthClientConfig};

server_config! {
    "service-filestore",
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Config {
        pub auth_server: AuthClientConfig,
        pub file_store: FileStoreConfig,
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileStoreConfig {
    known_services: HashSet<IpAddr>,
    pub read_role: Role,
    pub write_role: Role,
}

impl FileStoreConfig {
    pub fn address_is_service(&self, address: &IpAddr) -> bool {
        self.known_services.contains(address)
    }
}
