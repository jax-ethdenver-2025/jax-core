use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use iroh_blobs::{rpc::client::blobs::WrapOption, ticket::BlobTicket, util::SetTagOption};
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
    // Get endpoint and blobs from state
    let endpoint = state.endpoint().ok_or(ShareError::MissingEndpoint)?;
    let blobs = state.blobs().ok_or(ShareError::MissingBlobs)?;

    // Convert path string to PathBuf and get absolute path
    let path = PathBuf::from(request.path);
    let abs_path = std::path::absolute(&path).map_err(ShareError::Io)?;

    // Get the blobs client
    let blobs_client = blobs.client();

    // Add file to blob store
    let blob = blobs_client
        .add_from_path(
            abs_path.clone(),
            true,
            SetTagOption::Auto,
            WrapOption::NoWrap,
        )
        .await
        .map_err(ShareError::BlobOperation)?
        .finish()
        .await
        .map_err(ShareError::BlobOperation)?;

    // Create shareable ticket
    let node_id = endpoint.node_id();
    let ticket = BlobTicket::new(node_id.into(), blob.hash, blob.format)
        .map_err(ShareError::BlobOperation)?;

    let response = ShareResponse {
        ticket: ticket.to_string(),
        message: format!(
            "File '{}' has been added to the blob store",
            abs_path.display()
        ),
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
