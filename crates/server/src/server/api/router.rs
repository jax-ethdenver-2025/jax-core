use crate::server::api::{
    blob::{download, upload},
    hello::handler as hello_handler,
    tracker::{announce, discover},
};
use crate::server::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(hello_handler))
        .route("/blob", post(upload))
        .route("/blob/:hash", get(download))
        // Add tracker routes
        .route("/tracker/announce", post(announce))
        .route("/tracker/discover/:hash/:format", get(discover))
} 