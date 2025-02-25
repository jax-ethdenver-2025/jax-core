use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::server::state::ServerState;

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: String,
    node_id: String,
}

pub async fn handler(state: State<ServerState>) -> Json<ReadinessResponse> {
    let node_id = match state.node_id() {
        Ok(id) => id.to_string(),
        Err(_) => "unknown".to_string(),
    };

    Json(ReadinessResponse {
        status: "ok".to_string(),
        node_id,
    })
}
