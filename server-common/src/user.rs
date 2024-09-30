use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct User {
    name: Username,
    roles: HashSet<Role>,
}

impl User {
    pub fn new(name: Username, roles: HashSet<Role>) -> Self {
        Self { name, roles }
    }

    pub fn name(&self) -> &Username {
        &self.name
    }

    pub fn roles(&self) -> &HashSet<Role> {
        &self.roles
    }

    pub fn roles_mut(&mut self) -> &mut HashSet<Role> {
        &mut self.roles
    }
}

prae::define! {
    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    pub Username: String;
    adjust |username| *username = username.trim().to_lowercase();
    ensure |username| !username.is_empty() && username.chars().all(|ch| ch.is_alphanumeric() || ch == '_');
    plugins: [prae::impl_serde];
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

prae::define! {
    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    pub Role: String;
    adjust |role| *role = role.trim().to_lowercase();
    ensure |role| !role.is_empty();
    plugins: [prae::impl_serde];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
