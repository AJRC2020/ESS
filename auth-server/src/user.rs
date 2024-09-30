use std::collections::HashSet;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use password_hash::{PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};
use server_common::user::*;
use tracing::error;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct UserRecord {
    user: User,
    password_hash: String,
}

impl UserRecord {
    pub fn new(
        username: Username,
        password: String,
        roles: HashSet<Role>,
    ) -> Result<Self, password_hash::errors::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        Ok(Self {
            user: User::new(username, roles),
            password_hash,
        })
    }

    pub fn name(&self) -> &Username {
        self.user.name()
    }

    pub fn roles(&self) -> &HashSet<Role> {
        self.user.roles()
    }

    pub fn roles_mut(&mut self) -> &mut HashSet<Role> {
        self.user.roles_mut()
    }

    pub fn check_password(&self, password: &str) -> bool {
        let hash = match PasswordHash::new(&self.password_hash) {
            Ok(hash) => hash,
            Err(err) => {
                error!(?err, record = ?self, "Error parsing password hash");
                return false;
            }
        };

        match Argon2::default().verify_password(password.as_bytes(), &hash) {
            Ok(()) => true,
            Err(password_hash::Error::Password) => false,
            Err(err) => {
                error!(?err, record = ?self, "Error parsing password hash");
                false
            }
        }
    }
}
