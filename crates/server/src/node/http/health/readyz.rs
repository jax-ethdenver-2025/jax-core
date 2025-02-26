use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::node::State as NodeState;

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: String,
    node_id: String,
}

pub async fn handler(state: State<NodeState>) -> Json<ReadinessResponse> {
    let node_id = state.iroh_node_id();
    Json(ReadinessResponse {
        status: "ok".to_string(),
        node_id: node_id.to_string(),
    })
}
