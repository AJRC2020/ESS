use std::sync::OnceLock;

use axum::{
    async_trait,
    extract::FromRequestParts,
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt, TypedHeader,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use prae::Wrapper;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tracing::error;
use openssl::{
    sign::Verifier, 
    hash::MessageDigest, 
    pkey::PKey, 
    rsa::Rsa, 
};
use base64::{Engine, engine::general_purpose};


use crate::user::{Role, Username};
use crate::util::new_reqwest_client_from_certificates;

pub static ADMIN_ROLE: Lazy<Role> = Lazy::new(|| Role::new(String::from("admin")).unwrap());
pub static VIEWER_ROLE: Lazy<Role> = Lazy::new(|| Role::new(String::from("viewer")).unwrap());
pub static UPLOADER_ROLE: Lazy<Role> = Lazy::new(|| Role::new(String::from("uploader")).unwrap());
pub static SHARER_ROLE: Lazy<Role> = Lazy::new(|| Role::new(String::from("sharer")).unwrap());
pub static AUTH_CLIENT: OnceLock<AuthClient> = OnceLock::new();

const JWT_ALGORITHM: Algorithm = Algorithm::RS384;

const AUTH_SERVER_PUBLIC_KEY_PATH: &str = "cfg/auth-server.pem";
static AUTH_SERVER_PUBLIC_KEY: Lazy<DecodingKey> = Lazy::new(|| {
    DecodingKey::from_rsa_der(
        RsaPublicKey::read_public_key_pem_file(AUTH_SERVER_PUBLIC_KEY_PATH)
            .expect("Failed to load the public key for the authentication server from")
            .to_pkcs1_der()
            .expect("Failed to convert key to DER format")
            .as_bytes(),
    )
});

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    username: Username,
    #[serde(rename = "nbf")]
    not_before: i64,
    #[serde(rename = "exp")]
    expires: i64,
    public_key: String,
}

impl Claims {
    /// Creates a new `Claims` valid from the current instant and until `duration` has passed.
    pub fn create(username: Username, duration: Duration, public_key: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            username,
            not_before: now.unix_timestamp(),
            expires: (now + duration).unix_timestamp(),
            public_key,
        }
    }

    pub fn get_public_key(&self) -> &str {
        &self.public_key
    }

    /// Decodes and validates `Claims` from a JWT string.
    pub fn from_encoded(encoded: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(JWT_ALGORITHM);
        validation.set_required_spec_claims(&["exp", "nbf"]);
        validation.validate_nbf = true;

        jsonwebtoken::decode(encoded, &AUTH_SERVER_PUBLIC_KEY, &validation).map(|jwt| jwt.claims)
    }

    /// Encodes the `Claims` into a JWT, signed by the provided key.
    pub fn encode(&self, key: &EncodingKey) -> Result<String, jsonwebtoken::errors::Error> {
        jsonwebtoken::encode(&Header::new(JWT_ALGORITHM), &self, key)
    }

    pub fn username(&self) -> &Username {
        &self.username
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    #[tracing::instrument(skip(_state), ret)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingToken)?;

        let claim = Claims::from_encoded(bearer.token()).map_err(|err| {
            error!(?err, "Failed to decode or validate JWT");
            AuthError::InvalidToken
        })?;

        if !parts.headers.contains_key("Hash") {
            return Ok(claim)
        }

        let uri = parts.uri.path_and_query().map(|pq| pq.to_string()).unwrap_or_default();
        let uri = format!("https://localhost:8080{}", uri);

        let hash_string = parts.headers.get("Hash").expect("Failed getting signature").to_str().expect("Failed passing signature to string").to_string();
        let hash = general_purpose::STANDARD.decode(hash_string).expect("Failed decoding signature");
        
        let timestamp = parts.headers.get("Timestamp").expect("Failed getting timestamp").to_str().expect("Failed passing timestamp to string").to_string();
        let data = format!("{}+{}", timestamp, uri);

        let rsa = Rsa::public_key_from_pem(claim.get_public_key().as_bytes()).expect("Failed reading publ");
        let pkey = PKey::from_rsa(rsa).expect("Failed to get public key");

        let pkey_ref = pkey.as_ref();

        let mut verifier = Verifier::new(MessageDigest::sha256(), pkey_ref).expect("Failed to get verifier");
        verifier.update(data.as_bytes()).expect("Failed to update the verifier");

        if verifier.verify(&hash).expect("Failed to verify the signature") {
            Ok(claim)
        }
        else {
            Err(AuthError::InvalidSignature)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AuthError {
    InvalidToken,
    MissingToken,
    InvalidSignature
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::MissingToken => (StatusCode::BAD_REQUEST, "Missing token"),
            AuthError::InvalidSignature => (StatusCode::BAD_REQUEST, "Invalid signature")
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub struct AuthClient {
    client: reqwest::Client,
    authority: String,
}

impl AuthClient {
    pub fn new(name: &str, authority: String) -> anyhow::Result<Self> {
        let client = new_reqwest_client_from_certificates(name)?;
        Ok(Self { client, authority })
    }

    pub async fn user_has_role(
        &self,
        user: &Username,
        role: &Role,
    ) -> Result<bool, reqwest::Error> {
        let url = format!("https://{}/user/{}/is/{}", &self.authority, user, role);
        self.client.get(url).send().await?.json().await
    }

    pub async fn user_has_role_into_response(
        &self,
        user: &Username,
        role: &Role,
    ) -> Result<(), Response> {
        match self.user_has_role(user, role).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(StatusCode::FORBIDDEN.into_response()),
            Err(err) => {
                error!(
                    ?err,
                    "Failed to get role membership information from auth server"
                );
                Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}
