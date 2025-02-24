use axum::{routing, Router};
use http::header::{ACCEPT, ORIGIN};
use http::Method;
use tower_http::cors::{Any, CorsLayer};

mod hello;

pub use hello::handler as hello_handler;

use crate::app::AppState;

pub fn router(_state: AppState) -> Router<AppState> {
    let cors_layer = CorsLayer::new()
        .allow_methods(vec![Method::GET])
        .allow_headers(vec![ACCEPT, ORIGIN])
        .allow_origin(Any)
        .allow_credentials(false);

    Router::new()
        .route("/hello", routing::get(hello_handler))
        .layer(cors_layer)
}
