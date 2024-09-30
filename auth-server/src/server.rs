use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use jsonwebtoken::EncodingKey;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::Deserialize;
use serde_json::json;
use time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use zxcvbn::{zxcvbn, ZxcvbnError};

use crate::state::AppState;
use crate::user::UserRecord;
use server_common::auth::{Claims, ADMIN_ROLE, VIEWER_ROLE, UPLOADER_ROLE, SHARER_ROLE};
use server_common::user::{Role, Username};
use server_common::{unwrap_result_and_500_on_error, ORIGIN};

const AUTH_TOKEN_DURATION: Duration = Duration::hours(1);
const KEY_SIZE: usize = 2048;

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(config))
        .route("/db", get(db))
        .route("/user/login", post(login))
        .route("/user/register", post(register))
        .route("/user/:user/is/:role", get(user_in_role))
        .route("/user/:user/is/:role", put(add_role_to_user))
        .route("/user/:user/is/:role", delete(remove_role_from_user))
        .layer(
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE])
                .allow_origin(ORIGIN),
        )
}

async fn config(State(state): State<AppState>) -> String {
    format!("{:#?}", state.read().expect("poisoned lock"))
}

async fn db(State(state): State<AppState>) -> String {
    format!("{:#?}", state.read().expect("poisoned lock").db)
}

#[derive(Deserialize)]
struct LoginRequest {
    username: Username,
    password: String,
}

#[tracing::instrument(skip(state, request), ret)]
async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Response {
    let state = state.read().expect("poisoned lock");
    let user = match state
        .db
        .get_user_from_credentials(&request.username, &request.password)
    {
        Some(user) => user,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid credentials"})),
            )
                .into_response()
        }
    };

    create_jwt_response(user.name().clone(), &state.signing_key.jwt_key)
}

#[tracing::instrument(skip(state, request), ret)]
async fn register(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Response {
    let roles = {
        let state = state.read().expect("poisoned lock");
        let mut roles = state.config.default_roles().to_owned();

        // Make the first account into an admin account and make the account suitable for all roles.
        if state.db.len() == 0 {
            roles.insert(ADMIN_ROLE.clone());
            roles.insert(VIEWER_ROLE.clone());
            roles.insert(UPLOADER_ROLE.clone());
            roles.insert(SHARER_ROLE.clone());
        }

        roles
    };

    match zxcvbn(&request.password, &[request.username.as_ref()]) {
        Ok(entropy) => {
            if entropy.score() < 3 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "password too weak"})),
                )
                    .into_response();
            }
        }
        Err(ZxcvbnError::BlankPassword) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "blank password"})),
            )
                .into_response()
        }
        Err(err) => {
            error!(?err, "Error evaluating password");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let user = unwrap_result_and_500_on_error!(
        UserRecord::new(request.username, request.password, roles),
        "failed to hash password"
    );

    let username = user.name().clone();
    let mut state = state.write().expect("poisoned lock");
    match state.db.add_user(user) {
        Ok(true) => create_jwt_response(username, &state.signing_key.jwt_key),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({"error": "username already taken"})),
        )
            .into_response(),
        Err(err) => {
            error!(?err, "failed to save database");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

fn create_jwt_response(username: Username, key: &EncodingKey) -> Response {
    let mut rng = rand::thread_rng();
    let private_key = unwrap_result_and_500_on_error!(
        RsaPrivateKey::new(&mut rng, KEY_SIZE),
        "failed to generate key"
    );
    let public_key = RsaPublicKey::from(&private_key);

    let public_key_pem = unwrap_result_and_500_on_error!(
        public_key.to_public_key_pem(LineEnding::LF),
        "failed to convert key to PEM"
    );
    let private_key_pem = unwrap_result_and_500_on_error!(
        private_key.to_pkcs8_pem(LineEnding::LF),
        "failed to convert key to PEM"
    );

    let claims = Claims::create(username, AUTH_TOKEN_DURATION, public_key_pem);
    let jwt = unwrap_result_and_500_on_error!(claims.encode(key), "failed to encode JWT");

    Json(json!({"token": jwt, "private_key": *private_key_pem })).into_response()
}

#[tracing::instrument(skip(state), ret)]
async fn user_in_role(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path((username, role)): Path<(Username, Role)>,
) -> Response {
    let state = state.read().expect("poisoned lock");

    if !state.config.address_is_service(&addr.ip()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    match state.db.get_user(&username) {
        Some(user) => Json(user.roles().contains(role.as_ref())).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tracing::instrument(skip(state), ret)]
async fn add_role_to_user(
    State(state): State<AppState>,
    claims: Claims,
    Path((username, role)): Path<(Username, Role)>,
) -> StatusCode {
    {
        let state = state.read().expect("poisoned lock");
        match state.db.get_user(&claims.username()) {
            Some(user) => {
                if !user.roles().contains(ADMIN_ROLE.as_ref()) {
                    return StatusCode::UNAUTHORIZED;
                }
            }
            None => return StatusCode::BAD_REQUEST,
        };

        if !state.config.role_is_allowed(&role) {
            return StatusCode::BAD_REQUEST;
        }
    }

    match state
        .write()
        .expect("poisoned lock")
        .db
        .add_role_to_user(&username, role.clone())
    {
        Ok(true) => {
            info!(%username, %role, "Added role to user");
            StatusCode::OK
        }
        Ok(false) => StatusCode::NOT_FOUND,
        Err(err) => {
            error!(?err, "Failed to save database");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[tracing::instrument(skip(state), ret)]
async fn remove_role_from_user(
    State(state): State<AppState>,
    claims: Claims,
    Path((username, role)): Path<(Username, Role)>,
) -> StatusCode {
    match state
        .read()
        .expect("poisoned lock")
        .db
        .get_user(&claims.username())
    {
        Some(user) => {
            if !(user.roles().contains(ADMIN_ROLE.as_ref()) || user.name() == &username) {
                return StatusCode::UNAUTHORIZED;
            }
        }
        None => return StatusCode::BAD_REQUEST,
    };

    match state
        .write()
        .expect("poisoned lock")
        .db
        .remove_role_from_user(&username, &role)
    {
        Ok(true) => {
            info!(%username, %role, "Removed role from user");
            StatusCode::OK
        }
        Ok(false) => StatusCode::NOT_FOUND,
        Err(err) => {
            error!(?err, "Failed to save database");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
