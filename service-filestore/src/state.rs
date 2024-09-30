use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use crate::config::Config;
use server_common::auth::AuthClient;
use server_common::ServerConfig;

const FILESTORE_PATH: &str = "data/service-filestore/files/";

#[derive(Debug)]
pub struct State {
    pub config: Config,
}

pub type AppState = Arc<RwLock<State>>;

fn check_filename(file: &str) -> Result<PathBuf, io::Error> {
    let file_name = PathBuf::from(file);

    if file_name.is_absolute() || file_name.components().count() != 1 {
        return Err(io::Error::other(
            "Path isn't relative or has multiple components",
        ));
    }

    Ok(PathBuf::from(FILESTORE_PATH).join(file_name))
}

pub fn list_store() -> Result<Vec<String>, io::Error> {
    Ok(fs::read_dir(FILESTORE_PATH)?
        .filter_map(|file| file.ok())
        .filter_map(|file| file.file_name().into_string().ok())
        .collect())
}

pub fn read_store(file_name: &str) -> Result<Vec<u8>, io::Error> {
    let file_path = check_filename(file_name)?;

    let mut buffer = Vec::new();
    let mut file = File::open(file_path)?;
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

pub fn write_store(file_name: &str, file_content: &[u8]) -> Result<bool, io::Error> {
    let file_path = check_filename(file_name)?;

    if file_path.exists() {
        return Ok(false);
    }

    let mut file = File::create(file_path)?;
    file.write_all(file_content)?;

    Ok(true)
}

pub fn exists_store(file_name: &str) -> Result<bool, io::Error> {
    let file_path = check_filename(file_name)?;
    Ok(file_path.is_file())
}

pub static AUTH_CLIENT: OnceLock<AuthClient> = OnceLock::new();

pub fn get_state(config: Config) -> anyhow::Result<AppState> {
    // Ensure that the file store directory exists
    fs::create_dir_all(FILESTORE_PATH)?;

    AUTH_CLIENT
        .set(AuthClient::new(
            Config::name(),
            config.auth_server.authority(),
        )?)
        .ok()
        .expect("this should only get called once");

    Ok(Arc::new(RwLock::new(State { config })))
}
