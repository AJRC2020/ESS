use serde::Deserialize;

use server_common::user::Role;
use server_common::{server_config, AuthClientConfig};

server_config! {
    "service-fileshare",
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Config {
        pub auth_server: AuthClientConfig,
        pub file_share: FileShareConfig,
        pub filestore_server: FileStoreClientConfig,
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileShareConfig {
    pub share_role: Role,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileStoreClientConfig {
    host: String,
    port: u16,
}

impl FileStoreClientConfig {
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
