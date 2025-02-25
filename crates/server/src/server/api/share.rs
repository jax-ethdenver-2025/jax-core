use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use iroh_blobs::BlobFormat;
use iroh_blobs::ticket::BlobTicket;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::server::state::ServerState;

#[derive(Deserialize)]
pub struct ShareRequest {
    path: String,
}

#[derive(Serialize)]
pub struct ShareResponse {
    ticket: String,
    message: String,
}

pub async fn handler(
    State(state): State<ServerState>,
    Json(request): Json<ShareRequest>,
) -> Result<impl IntoResponse, ShareError> {
    // Convert path string to PathBuf and get absolute path
    let path = PathBuf::from(request.path);
    let abs_path = std::path::absolute(&path).map_err(ShareError::Io)?;

    // Read file content
    let content = tokio::fs::read(&abs_path).await.map_err(ShareError::Io)?;
    
    // TODO: Determine format based on file extension (could be more sophisticated)
    let format = BlobFormat::Raw;
    
    // Use blob_service directly
    let hash = state.blob_service().store_blob(content, format)
        .await
        .map_err(ShareError::BlobOperation)?;
    
    // Create ticket
    let node_id = state.endpoint().node_id();
    let ticket = BlobTicket::new(node_id.into(), hash, format)
        .map_err(ShareError::BlobOperation)?;
    
    let response = ShareResponse {
        ticket: ticket.to_string(),
        message: format!("File '{}' has been added to the blob store", abs_path.display()),
    };

    Ok((axum::http::StatusCode::OK, Json(response)))
}

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("missing endpoint")]
    MissingEndpoint,
    #[error("missing blobs store")]
    MissingBlobs,
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("blob operation failed: {0}")]
    BlobOperation(anyhow::Error),
}

impl IntoResponse for ShareError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ShareError::MissingEndpoint => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Server endpoint not configured".to_string(),
            ),
            ShareError::MissingBlobs => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Blob store not configured".to_string(),
            ),
            ShareError::Io(e) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("File error: {}", e),
            ),
            ShareError::BlobOperation(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Blob operation failed: {}", e),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
