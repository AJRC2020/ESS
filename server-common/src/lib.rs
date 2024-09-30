pub mod auth;
mod cli;
mod config;
pub mod user;
pub mod util;

use std::fs::File;
use std::io::Read;
use std::net::{Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::Context;
use axum::http::HeaderValue;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use serde::de::DeserializeOwned;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

pub use crate::cli::ServerArgs;
pub use crate::config::{AuthClientConfig, GeneralConfig, ServerConfig};

pub const ORIGIN: HeaderValue = HeaderValue::from_static("https://localhost:8080");

pub mod prelude {
    pub use crate::{server_args, server_config, ServerConfig};
    pub use clap::{self, Parser};
}

fn tracing_setup() -> anyhow::Result<()> {
    let filter = if std::env::var_os(EnvFilter::DEFAULT_ENV).is_some() {
        EnvFilter::builder()
            .with_default_directive(tracing::Level::DEBUG.into())
            .from_env()
            .context("Invalid logging configuration")?
    } else {
        EnvFilter::builder().parse("debug,hyper=info").unwrap()
    };

    let collector = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .finish();

    tracing::subscriber::set_global_default(collector).expect("failed to set global logger");
    Ok(())
}

fn load_config<C: ServerConfig + DeserializeOwned>(path: &Path) -> anyhow::Result<C> {
    let mut file = File::open(path)
        .with_context(|| format!("failed to open config file '{}'", path.display()))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("failed to read config file '{}'", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("failed to deserialize config file '{}'", path.display()))
}

async fn get_tls_config(name: &str) -> anyhow::Result<RustlsConfig> {
    let cert_dir = PathBuf::from("cfg/tls");
    let cert_path = cert_dir.join(format!("{}.cert", name));
    let key_path = cert_dir.join(format!("{}.key", name));
    RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("failed to load TLS certificate")
}

pub fn server_main<C: ServerConfig + DeserializeOwned, S: Clone + Send + Sync + 'static>(
    args: &impl ServerArgs,
    router: Router<S>,
    get_state: impl FnOnce(C) -> anyhow::Result<S>,
) -> anyhow::Result<()> {
    tracing_setup()?;

    let config: C = load_config(args.config_path())?;
    let port = args.port().unwrap_or(config.port());
    let addr = SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), port);

    let state = get_state(config)?;
    Runtime::new()?.block_on(async {
        axum_server::bind_rustls(
            addr,
            get_tls_config(C::name())
                .await
                .context("failed to get rustls config")?,
        )
        .serve(
            router
                .with_state(state)
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .context("HTTP server error")
    })
}
