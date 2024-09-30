use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use reqwest::tls::{Certificate, Identity};
use reqwest::Client;

pub fn new_reqwest_client_from_certificates(name: &str) -> anyhow::Result<Client> {
    let cert_dir = PathBuf::from("cfg/tls");
    let root_ca_cert = fs::read(cert_dir.join("root_ca.cert")).context("failed to read root CA")?;
    let cert = fs::read(cert_dir.join(format!("{}.cert", name)))
        .context("failed to read TLS certificate")?;
    let mut cert_and_key =
        fs::read(cert_dir.join(format!("{}.key", name))).context("failed to read TLS key")?;
    cert_and_key.extend(cert);

    Client::builder()
        .add_root_certificate(Certificate::from_pem(&root_ca_cert).context("invalid CA")?)
        .identity(Identity::from_pem(&cert_and_key).context("invalid certificate")?)
        .build()
        .context("failed to build reqwest client")
}

#[macro_export]
macro_rules! unwrap_result_and_500_on_error {
    ($e: expr, $err_msg: expr) => {
        match $e {
            Ok(val) => val,
            Err(err) => {
                ::tracing::error!(?err, $err_msg);
                use ::axum::response::IntoResponse;
                return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    };
}
