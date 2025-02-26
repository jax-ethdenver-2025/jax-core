use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use iroh::NodeId;
use iroh_blobs::get::Stats;
use iroh_blobs::{BlobFormat, HashAndFormat};
use iroh_blobs::ticket::BlobTicket;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use crate::node::State as NodeState;
use crate::server::utils::ephemeral_endpoint;

#[derive(Deserialize)]
pub struct ProbeRequest {
    ticket: String,
}

#[derive(Serialize)]
pub struct ProbeResponse {
    node_id: NodeId,
    content: HashAndFormat,
    stats: Stats,
    elapsed: std::time::Duration,
}

pub async fn handler(
    State(state): State<NodeState>,
    Json(request): Json<ProbeRequest>,
) -> Result<impl IntoResponse, ProbeError> {
    let tracker_service = state.tracker_service();
    // Convert path string to PathBuf and get absolute path
    let ticket: BlobTicket = request.ticket
        .parse()
        .map_err(|_| ProbeError::InvalidTicket(request.ticket.clone()))?;

    let node_id = ticket.node_addr().node_id;
    let hash = ticket.hash();
    let format = ticket.format();

    let content = HashAndFormat { 
        hash, 
        format,
    };

    let start_time = Instant::now();
    let stats = tracker_service.probe_node(node_id, content).await?;
    let elapsed = start_time.elapsed();

    let response = ProbeResponse {
        node_id,
        content,
        stats,
        elapsed,
    };

    Ok((axum::http::StatusCode::OK, Json(response)))
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
    #[error("invalid ticket: {0}")]
    InvalidTicket(String),
}

impl IntoResponse for ProbeError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ProbeError::Default(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Server endpoint not configured".to_string(),
            ),
            ProbeError::InvalidTicket(_) => (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid ticket".to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
