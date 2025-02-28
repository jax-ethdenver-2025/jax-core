use axum::{routing, Router};
use http::header::{ACCEPT, ORIGIN};
use http::Method;
use tower_http::cors::{Any, CorsLayer};

mod create_pool;
mod list;
mod pools;
mod probe;
mod pull;
mod query;
mod share;

pub use create_pool::handler as create_pool_handler;
pub use list::handler as list_handler;
pub use pools::handler as pools_handler;
pub use probe::handler as probe_handler;
pub use pull::handler as pull_handler;
pub use query::handler as query_handler;
pub use share::handler as share_handler;

use crate::node::State as NodeState;

pub fn router(state: NodeState) -> Router<NodeState> {
    let cors_layer = CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![ACCEPT, ORIGIN])
        .allow_origin(Any)
        .allow_credentials(false);

    Router::new()
        .route("/share", routing::post(share_handler))
        .route("/list", routing::get(list_handler))
        .route("/probe", routing::post(probe_handler))
        .route("/query/:hash", routing::get(query_handler))
        .route("/pool", routing::post(create_pool_handler))
        .route("/pools", routing::get(pools_handler))
        .route("/pull/:hash", routing::get(pull_handler))
        .with_state(state)
        .layer(cors_layer)
}
