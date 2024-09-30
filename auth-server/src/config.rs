use std::collections::HashSet;
use std::net::IpAddr;

use serde::Deserialize;

use server_common::auth::ADMIN_ROLE;
use server_common::server_config;
use server_common::user::Role;

server_config! {
    "auth-server",
    #[derive(Clone, Debug, Deserialize)]
    pub struct Config {
        pub authenticator: AuthConfig,
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthConfig {
    allowed_roles: AllowedRoleSet,
    default_roles: HashSet<Role>,
    known_services: HashSet<IpAddr>,
}

impl AuthConfig {
    pub fn address_is_service(&self, address: &IpAddr) -> bool {
        self.known_services.contains(address)
    }

    pub fn default_roles(&self) -> &HashSet<Role> {
        &self.default_roles
    }

    pub fn role_is_allowed(&self, role: &Role) -> bool {
        self.allowed_roles.contains(role)
    }
}

prae::define! {
    #[derive(Clone, Debug)]
    AllowedRoleSet: HashSet<Role>;
    validate(&'static str) |set|
        set.contains(ADMIN_ROLE.as_ref())
            .then_some(())
            .ok_or("`allowed-roles` must contain the `admin` role");
    plugins: [prae::impl_serde];
}

impl AllowedRoleSet {
    fn contains(&self, role: &Role) -> bool {
        self.0.contains(role.as_ref())
    }
}
