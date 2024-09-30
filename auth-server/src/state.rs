use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use jsonwebtoken::EncodingKey;
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use server_common::user::{Role, Username};
use thiserror::Error;
use tracing::info;

use crate::config::{AuthConfig, Config};
use crate::user::UserRecord;

const DB_PATH: &str = "data/auth-server/db.json";
const KEY_SIZE: usize = 2048;
const PRIVATE_KEY_PATH: &str = "cfg/auth-server-private.pem";
const PUBLIC_KEY_PATH: &str = "cfg/auth-server.pem";

#[derive(Debug)]
pub struct State {
    pub config: AuthConfig,
    pub db: Database,
    pub signing_key: Key,
}

pub type AppState = Arc<RwLock<State>>;

pub struct Key {
    pub key: RsaPrivateKey,
    pub jwt_key: EncodingKey,
}

impl From<RsaPrivateKey> for Key {
    fn from(key: RsaPrivateKey) -> Key {
        let pkcs1_der = key
            .to_pkcs1_der()
            .expect("Failed to convert key to DER format");
        Key {
            key,
            jwt_key: EncodingKey::from_rsa_der(pkcs1_der.as_bytes()),
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key")
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Database {
    users: HashMap<Username, UserRecord>,
}

impl Database {
    pub fn get_user(&self, username: &Username) -> Option<&UserRecord> {
        self.users.get(username.as_ref())
    }

    pub fn get_user_from_credentials(
        &self,
        username: &Username,
        password: &str,
    ) -> Option<&UserRecord> {
        self.users
            .get(username.as_ref())
            .and_then(|record| record.check_password(password).then_some(record))
    }

    pub fn len(&self) -> usize {
        self.users.len()
    }

    pub fn add_user(&mut self, user: UserRecord) -> Result<bool, SaveError> {
        if self.users.contains_key(user.name().as_ref()) {
            return Ok(false);
        }

        if self.users.insert(user.name().clone(), user).is_some() {
            panic!("user shouldn't exist");
        }
        self.save()?;
        Ok(true)
    }

    pub fn add_role_to_user(&mut self, username: &Username, role: Role) -> Result<bool, SaveError> {
        match self.users.get_mut(username.as_ref()) {
            Some(user) => {
                user.roles_mut().insert(role);
                self.save().and(Ok(true))
            }
            None => Ok(false),
        }
    }

    pub fn remove_role_from_user(
        &mut self,
        username: &Username,
        role: &Role,
    ) -> Result<bool, SaveError> {
        match self.users.get_mut(username.as_ref()) {
            Some(user) => {
                let removed = Ok(user.roles_mut().remove(role.as_ref()));
                self.save().and(removed)
            }
            None => Ok(false),
        }
    }

    pub fn save(&self) -> Result<(), SaveError> {
        fs::create_dir_all(Path::new(DB_PATH).parent().unwrap())?;

        let mut file = File::create(DB_PATH)?;
        serde_json::to_writer_pretty(&mut file, self)?;
        Ok(())
    }
}

fn get_signing_key() -> anyhow::Result<RsaPrivateKey> {
    let private_key_path = Path::new(PRIVATE_KEY_PATH);
    let private_key = if private_key_path.is_file() {
        info!("Loading signing key from '{}'", PRIVATE_KEY_PATH);
        RsaPrivateKey::read_pkcs8_pem_file(private_key_path)?
    } else {
        info!("Generating new signing key");
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, KEY_SIZE)?;
        key.write_pkcs8_pem_file(private_key_path, LineEnding::LF)?;
        key
    };

    let public_key = RsaPublicKey::from(&private_key);

    let public_key_path = Path::new(PUBLIC_KEY_PATH);
    if !public_key_path.is_file()
        || RsaPublicKey::read_public_key_pem_file(public_key_path)? != public_key
    {
        info!("Writing public key to '{}'", PUBLIC_KEY_PATH);
        public_key.write_public_key_pem_file(public_key_path, LineEnding::LF)?;
    }

    Ok(private_key)
}

pub fn get_state(config: Config) -> anyhow::Result<AppState> {
    let signing_key = get_signing_key()?.into();

    let db = match File::open(DB_PATH) {
        Ok(file) => serde_json::from_reader(file).context("Failed to deserialize db file")?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => Default::default(),
        Err(err) => return Err(err).context("Failed to open db file"),
    };

    Ok(Arc::new(RwLock::new(State {
        config: config.authenticator,
        db,
        signing_key,
    })))
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("IO error saving database: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error saving database: {0}")]
    Json(#[from] serde_json::Error),
}
