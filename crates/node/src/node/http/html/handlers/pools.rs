use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use alloy::primitives::Address;
use iroh::NodeId;
use iroh_blobs::Hash;

use crate::node::State as NodeState;

#[derive(Template)]
#[template(path = "pools.html")]
struct PoolsTemplate {
    pools: Vec<(Address, Hash, Vec<(NodeId, f64)>)>,
}

#[axum::debug_handler]
pub async fn pools_handler(
    State(state): State<NodeState>,
) -> impl IntoResponse {
    let pool_map = state.tracker()
        .list_pools_with_trust()
        .await
        .unwrap_or_default();

    let pools: Vec<(Address, Hash, Vec<(NodeId, f64)>)> = pool_map
        .into_iter()
        .map(|(key, peers)| {
            (
                key.address,
                key.hash,
                peers.into_iter().collect()
            )
        })
        .collect();

    PoolsTemplate { pools }
} 