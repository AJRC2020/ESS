use axum::extract::{Path, State};
use axum::http::header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::CorsLayer;
use tracing::error;

use crate::link::LinkCode;
use crate::state::{AppState, AUTH_CLIENT, CLIENT};
use server_common::auth::Claims;
use server_common::{unwrap_result_and_500_on_error, ORIGIN};

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(config))
        .route("/db", get(db))
        .route("/links", get(user_links))
        .route("/link", put(add_link))
        .route("/link/:code", get(file_of_link))
        .route("/link/:code", delete(delete_link))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::PUT, Method::DELETE, Method::OPTIONS])
                .allow_headers([ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE])
                .allow_origin(ORIGIN),
        )
}

async fn config(State(state): State<AppState>) -> String {
    format!("{:#?}", state.read().expect("poisoned lock").config)
}

async fn db(State(state): State<AppState>) -> String {
    format!("{:#?}", state.read().expect("poisoned lock").db)
}

// Get a list of Links for a user (by Username string)
#[tracing::instrument(skip(state), ret)]
async fn user_links(State(state): State<AppState>, claims: Claims) -> Response {
    let username = claims.username();
    let state = state.read().expect("poisoned lock");
    let links = state.db.get_file_links_for_user(username);
    Json(json!(links)).into_response()
}

// Get a file name from a link then return the file in the response (from filestore service)
#[tracing::instrument(skip(state), ret)]
async fn file_of_link(State(state): State<AppState>, Path(code): Path<LinkCode>) -> Response {
    let (link, authority) = {
        let state = state.read().expect("poisoned lock");
        (state.db.get_link_by_code(&code).map(|v| v.to_owned()), state.config.filestore_server.authority())
    };
    if let Some(link) = link {
        let file_name = link.file_name();
        match CLIENT
            .get()
            .unwrap()
            .get(format!(
                "https://{}/file-shared/{}",
                authority, &file_name
            ))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let headers = response.headers().to_owned();
                match response.bytes().await {
                    Ok(body) => (status, headers, body).into_response(),
                    Err(err) => {
                        error!(?err, "Error in response");
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                }
            }
            Err(err) => {
                error!(?err, "Failed to get file");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(Debug, Deserialize)]
struct AddLinkRequest {
    file_name: String,
}

// Post a new Link to the database
#[tracing::instrument(skip(state), ret)]
async fn add_link(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<AddLinkRequest>,
) -> Response {
    let (role, filestore_authority) = {
        let state = state.read().expect("poisoned lock");
        (
            state.config.file_share.share_role.clone(),
            state.config.filestore_server.authority(),
        )
    };

    if let Err(response) = AUTH_CLIENT
        .get()
        .unwrap()
        .user_has_role_into_response(claims.username(), &role)
        .await
    {
        return response;
    }

    match CLIENT
        .get()
        .unwrap()
        .get(format!(
            "https://{}/file-exists/{}",
            filestore_authority, &request.file_name
        ))
        .send()
        .await
    {
        Ok(resp) => match resp.json::<bool>().await {
            Ok(exists) => {
                if !exists {
                    return StatusCode::NOT_FOUND.into_response();
                }
            }
            Err(err) => {
                error!(?err, "Failed to check if file exists");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        },
        Err(err) => {
            error!(?err, "Failed to check if file exists");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let code = unwrap_result_and_500_on_error!(
        state
            .write()
            .expect("poisoned lock")
            .db
            .add_link(claims.username().to_owned(), request.file_name),
        "error saving database"
    );

    Json(code).into_response()
}

// Delete a link with code
#[tracing::instrument(skip(state), ret)]
async fn delete_link(
    State(state): State<AppState>,
    claims: Claims,
    Path(code): Path<LinkCode>,
) -> Response {
    let link_owner = match state
        .read()
        .expect("poisoned lock")
        .db
        .get_link_by_code(&code)
    {
        Some(link) => link.username().to_owned(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if claims.username() != &link_owner {
        match AUTH_CLIENT
            .get()
            .unwrap()
            .user_has_role(claims.username(), &server_common::auth::ADMIN_ROLE)
            .await
        {
            Ok(true) => {} // admin users can remove any link
            Ok(false) => return StatusCode::FORBIDDEN.into_response(),
            Err(err) => {
                error!(
                    ?err,
                    "Failed to get role membership information from auth server"
                );
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    match state.write().expect("poisoned lock").db.delete_link(code) {
        Ok(true) => StatusCode::OK,
        Ok(false) => StatusCode::BAD_REQUEST,
        Err(err) => {
            error!(?err, "Error saving database");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
    .into_response()
}
