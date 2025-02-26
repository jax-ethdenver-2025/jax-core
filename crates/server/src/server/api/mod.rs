use axum::{routing, Router};
use http::header::{ACCEPT, ORIGIN};
use http::Method;
use tower_http::cors::{Any, CorsLayer};

mod list_blobs;
mod share;
mod probe;
mod query;

pub use list_blobs::handler as list_blobs_handler;
pub use share::handler as share_handler;
pub use probe::handler as probe_handler;
pub use query::handler as query_handler;

use crate::server::state::ServerState;

pub fn router(state: ServerState) -> Router<ServerState> {
    let cors_layer = CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![ACCEPT, ORIGIN])
        .allow_origin(Any)
        .allow_credentials(false);

    Router::new()
        .route("/share", routing::post(share_handler))
        .route("/blobs", routing::get(list_blobs_handler))
        .route("/probe", routing::post(probe_handler))
        .route("/query", routing::post(query_handler))
        .with_state(state)
        .layer(cors_layer)
}
