use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use iroh_blobs::BlobFormat;
use iroh_blobs::ticket::BlobTicket;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::node::State as NodeState;

#[derive(Deserialize)]
pub struct ShareRequest {
    path: String,
}

#[derive(Serialize)]
pub struct ShareResponse {
    ticket: String,
    hash: String,
    message: String,
}

pub async fn handler(
    State(state): State<NodeState>,
    Json(request): Json<ShareRequest>,
) -> Result<impl IntoResponse, ShareError> {
    // Convert path string to PathBuf and get absolute path
    let path = PathBuf::from(request.path);
    let abs_path = std::path::absolute(&path).map_err(ShareError::Io)?;
    let tracker_service = state.tracker_service();

    // Read file content
    let content = tokio::fs::read(&abs_path).await.map_err(ShareError::Io)?;
    
    // Determine format based on file extension
    let format = BlobFormat::Raw;
    
    // Use blob_service directly to store the blob
    let hash = state.blob_service().store_blob(content)
        .await
        .map_err(ShareError::BlobOperation)?;
    
    // Create ticket
    let node_id = state.endpoint().node_id();
    let ticket = BlobTicket::new(node_id.into(), hash, format)
        .map_err(ShareError::BlobOperation)?;

    tracker_service.broadcast_ticket(ticket.clone()).await?;

    let hash_str = hash.to_string();
    
    let response = ShareResponse {
        ticket: ticket.to_string(),
        hash: hash_str,
        message: format!("File '{}' has been added to the blob store and announced to the network", abs_path.display()),
    };

    Ok((axum::http::StatusCode::OK, Json(response)))
}

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("blob operation failed: {0}")]
    BlobOperation(anyhow::Error),
}

impl IntoResponse for ShareError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ShareError::Io(e) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("File error: {}", e),
            ),
            ShareError::BlobOperation(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Blob operation failed: {}", e),
            ),
            ShareError::Default(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error: {}", e),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
