use axum::Json;
use serde::Serialize;

use crate::server::state::ServerState;

#[derive(Serialize)]
pub struct LivenessResponse {
    status: String,
}

pub async fn handler(_state: axum::extract::State<ServerState>) -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok".to_string(),
    })
}
