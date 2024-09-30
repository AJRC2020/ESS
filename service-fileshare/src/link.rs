use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

use server_common::user::Username;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Link {
    username: Username,
    file_name: String,
}

impl Link {
    pub fn new(username: Username, file_name: String) -> Self {
        Self {
            username,
            file_name,
        }
    }

    pub fn username(&self) -> &Username {
        &self.username
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }
}

const LINK_CODE_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct LinkCode(String);

impl LinkCode {
    pub fn new() -> LinkCode {
        let rng = rand::thread_rng();
        Self(
            rng.sample_iter(&Alphanumeric)
                .take(LINK_CODE_SIZE)
                .map(char::from)
                .collect(),
        )
    }
}
