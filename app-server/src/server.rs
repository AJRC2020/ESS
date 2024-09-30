use std::str::FromStr;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, AUTHORIZATION, CONNECTION, CONTENT_TYPE,
};
use axum::http::uri::Authority;
use axum::http::{Method, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::error;

use crate::state::AppState;
use server_common::ORIGIN;

macro_rules! proxy {
    ($name: ident, $method: ident, $service_config_entry: ident) => {
        async fn $name(State(state): State<AppState>, request: Request<Body>) -> Response {
            let uri = match Uri::builder()
                .scheme("https")
                .authority(
                    Authority::from_str(&state.config.$service_config_entry.authority())
                        .expect("SocketAddr.to_string() should be a valid authority"),
                )
                .path_and_query(request.uri().path())
                .build()
            {
                Ok(uri) => uri,
                Err(err) => {
                    error!(?err, "Error building URI");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            match state
                .client
                .$method(uri.to_string())
                .headers(request.headers().to_owned())
                .body(reqwest::Body::from(request.into_body()))
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
                    error!(?err, "Error sending request");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    };
}

proxy!(filestore_get, get, filestore_server);
proxy!(filestore_put, put, filestore_server);
proxy!(fileshare_get, get, fileshare_server);
proxy!(fileshare_put, put, fileshare_server);
proxy!(fileshare_delete, delete, fileshare_server);

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(config))
        .route("/files", get(filestore_get))
        .route("/files/:file", get(filestore_get))
        .route("/files/:file", put(filestore_put))
        .route("/links", get(fileshare_get))
        .route("/link", put(fileshare_put))
        .route("/link/:code", get(fileshare_get))
        .route("/link/:code", delete(fileshare_delete))
        .fallback_service(ServeDir::new("www"))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::PUT, Method::DELETE, Method::OPTIONS])
                .allow_headers([
                    ACCEPT,
                    ACCEPT_ENCODING,
                    ACCEPT_LANGUAGE,
                    AUTHORIZATION,
                    CONNECTION,
                    CONTENT_TYPE,
                ])
                .allow_origin(ORIGIN),
        )
}

#[tracing::instrument]
async fn config(State(state): State<AppState>) -> String {
    format!("{:#?}", state.config)
}
