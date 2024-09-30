use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use tower_http::cors::CorsLayer;
use tracing::error;

use crate::state::{exists_store, list_store, read_store, write_store, AppState, AUTH_CLIENT};
use server_common::auth::Claims;
use server_common::ORIGIN;

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(config))
        .route("/files", get(list))
        .route("/files/:file", get(read))
        .route("/files/:file", put(write))
        .route("/file-exists/:file", get(exists))
        .route("/file-shared/:file", get(read_shared))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::PUT, Method::OPTIONS])
                .allow_headers([ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE])
                .allow_origin(ORIGIN),
        )
}

#[tracing::instrument]
async fn config(State(state): State<AppState>) -> String {
    format!("{:#?}", state.read().expect("poisoned lock").config)
}

#[tracing::instrument(skip(state), ret)]
async fn list(State(state): State<AppState>, claims: Claims) -> Response {
    let role = state
        .read()
        .expect("poisoned lock")
        .config
        .file_store
        .read_role
        .clone();
    if let Err(response) = AUTH_CLIENT
        .get()
        .unwrap()
        .user_has_role_into_response(claims.username(), &role)
        .await
    {
        return response;
    }

    match list_store() {
        Ok(files) => (StatusCode::OK, Json(files)).into_response(),
        Err(err) => {
            error!(?err, "Failed to get list of files");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

#[tracing::instrument(skip(state), ret)]
async fn read(State(state): State<AppState>, claims: Claims, Path(file): Path<String>) -> Response {
    let role = state
        .read()
        .expect("poisoned lock")
        .config
        .file_store
        .read_role
        .clone();
    if let Err(response) = AUTH_CLIENT
        .get()
        .unwrap()
        .user_has_role_into_response(claims.username(), &role)
        .await
    {
        return response;
    }

    match read_store(&file) {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", &file),
            )
            .body(Body::from(content))
            .unwrap()
            .into_response(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(err) => {
            error!(?err, "Failed to read file from store");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[tracing::instrument(skip(state), ret)]
async fn read_shared(State(state): State<AppState>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Path(file): Path<String>) -> Response {
    if !state.read().expect("poisoned lock").config.file_store.address_is_service(&addr.ip()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    match read_store(&file) {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", &file),
            )
            .body(Body::from(content))
            .unwrap()
            .into_response(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(err) => {
            error!(?err, "Failed to read file from store");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[tracing::instrument(skip(state), ret)]
async fn write(
    State(state): State<AppState>,
    claims: Claims,
    Path(file): Path<String>,
    contents: String,
) -> Response {
    let role = state
        .read()
        .expect("poisoned lock")
        .config
        .file_store
        .write_role
        .clone();
    if let Err(response) = AUTH_CLIENT
        .get()
        .unwrap()
        .user_has_role_into_response(claims.username(), &role)
        .await
    {
        return response;
    }

    let body = contents.as_bytes().to_vec();

    match write_store(&file, &body) {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => (StatusCode::CONFLICT, "File already exists").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[tracing::instrument(skip(state), ret)]
async fn exists(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(file): Path<String>,
) -> Response {
    if !state
        .read()
        .expect("poisoned lock")
        .config
        .file_store
        .address_is_service(&addr.ip())
    {
        return StatusCode::FORBIDDEN.into_response();
    }

    match exists_store(&file) {
        Ok(res) => Json(res).into_response(),
        Err(err) => {
            error!(?err, "Failed to check if file exists");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
