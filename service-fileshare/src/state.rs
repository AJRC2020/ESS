use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::Config;
use crate::link::{Link, LinkCode};
use server_common::auth::AuthClient;
use server_common::user::Username;
use server_common::util::new_reqwest_client_from_certificates;
use server_common::ServerConfig;

const DB_PATH: &str = "data/service-fileshare/links/db.json";

#[derive(Debug)]
pub struct State {
    pub config: Config,
    pub db: Database,
}

pub type AppState = Arc<RwLock<State>>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Database {
    links: HashMap<LinkCode, Link>,
}

impl Database {
    pub fn add_link(
        &mut self,
        username: Username,
        file_name: String,
    ) -> Result<LinkCode, SaveError> {
        let mut code;
        loop {
            code = LinkCode::new();
            if !self.links.contains_key(&code) {
                break;
            }
        }
        self.links
            .insert(code.clone(), Link::new(username, file_name));
        self.save()?;
        Ok(code)
    }

    pub fn delete_link(&mut self, code: LinkCode) -> Result<bool, SaveError> {
        if self.links.remove(&code).is_some() {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_file_links_for_user(&self, username: &Username) -> HashMap<LinkCode, &Link> {
        let mut links = HashMap::new();
        for (code, link) in self.links.iter() {
            if link.username() == username {
                links.insert(code.clone(), link);
            }
        }
        links
    }

    pub fn get_link_by_code(&self, code: &LinkCode) -> Option<&Link> {
        self.links.get(&code)
    }

    pub fn save(&self) -> Result<(), SaveError> {
        fs::create_dir_all(Path::new(DB_PATH).parent().unwrap())?;

        let mut file = File::create(DB_PATH)?;
        serde_json::to_writer_pretty(&mut file, self)?;
        Ok(())
    }
}

pub static AUTH_CLIENT: OnceLock<AuthClient> = OnceLock::new();
pub static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn get_state(config: Config) -> anyhow::Result<AppState> {
    AUTH_CLIENT
        .set(AuthClient::new(
            Config::name(),
            config.auth_server.authority(),
        )?)
        .ok()
        .expect("this should only get called once");

    CLIENT
        .set(new_reqwest_client_from_certificates("service-fileshare")?)
        .ok()
        .expect("this should only get called once");

    let db = match File::open(DB_PATH) {
        Ok(file) => serde_json::from_reader(file).context("Failed to deserialize db file")?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => Default::default(),
        Err(err) => return Err(err).context("Failed to open db file"),
    };

    Ok(Arc::new(RwLock::new(State { config, db })))
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("IO error saving database: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error saving database: {0}")]
    Json(#[from] serde_json::Error),
}
