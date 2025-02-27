use std::time::Instant;

use axum::extract::Json;
use axum::response::{IntoResponse, Response};
use iroh::NodeId;
use iroh_blobs::get::Stats;
use iroh_blobs::{Hash, BlobFormat, HashAndFormat};
use iroh_blobs::ticket::BlobTicket;
use serde::{Deserialize, Serialize};

use crate::node::tracker::Tracker;

#[derive(Deserialize)]
pub struct ProbeRequest {
    peer_id: NodeId,
    hash: Hash,
}

#[derive(Serialize)]
pub struct ProbeResponse {
    node_id: NodeId,
    content: HashAndFormat,
    stats: Stats,
    elapsed: std::time::Duration,
}

pub async fn handler(
    Json(request): Json<ProbeRequest>,
) -> Result<impl IntoResponse, ProbeError> {
    let ticket = BlobTicket::new(request.peer_id.into(), request.hash, BlobFormat::Raw)?;
    let node_id = ticket.node_addr().node_id;

    let hash = ticket.hash();
    let format = ticket.format();

    let content = HashAndFormat { 
        hash, 
        format,
    };

    let start_time = Instant::now();
    let stats = Tracker::probe_node(ticket).await?;
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
}

impl IntoResponse for ProbeError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ProbeError::Default(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Server endpoint not configured".to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
