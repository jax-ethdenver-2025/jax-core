use axum::{routing, Router};
use http::header::{ACCEPT, ORIGIN};
use http::Method;
use tower_http::cors::{Any, CorsLayer};

mod hello;
mod share;

pub use hello::handler as hello_handler;
pub use share::handler as share_handler;

use crate::server::state::ServerState;

pub fn router(state: ServerState) -> Router<ServerState> {
    let cors_layer = CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![ACCEPT, ORIGIN])
        .allow_origin(Any)
        .allow_credentials(false);

    Router::new()
        .route("/hello", routing::get(hello_handler))
        .route("/share", routing::post(share_handler))
        .with_state(state)
        .layer(cors_layer)
}
