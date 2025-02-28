use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use iroh::NodeId;
use alloy::primitives::Address;

use crate::node::State as NodeState;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    node_id: NodeId,
    eth_address: Address,
}

#[axum::debug_handler]
pub async fn index_handler(
    State(state): State<NodeState>,
) -> impl IntoResponse {
    let node_id = state.iroh_node_id();
    let eth_address = state.eth_address();
    
    IndexTemplate {
        node_id,
        eth_address,
    }
} 