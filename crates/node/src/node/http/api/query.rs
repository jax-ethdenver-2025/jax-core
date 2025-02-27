use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use iroh_blobs::rpc::client::blobs::BlobStatus;
use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};

use crate::node::State as NodeState;

#[derive(Deserialize)]
pub struct QueryLocationsRequest {
    hash: Hash,
}

#[derive(Serialize)]
pub struct QueryLocationsResponse {
    local: bool,
    nodes: Vec<iroh::NodeId>,
    message: String,
}

pub async fn handler(
    State(state): State<NodeState>,
    Json(request): Json<QueryLocationsRequest>,
) -> Result<impl IntoResponse, QueryError> {
    let nodes = state.tracker().get_peers_for_hash(request.hash)
        .await
        .map_err(QueryError::Default)?;

    let blob_status = state.blobs_service().get_inner_blobs().client().status(request.hash.clone()).await?;
    let local = match blob_status {
        BlobStatus::Complete { .. } => true,
        _ => false,
    };

    let response = QueryLocationsResponse {
        local,
        nodes: nodes.clone(),
        message: format!("Found {} nodes hosting blob {}", nodes.len(), request.hash),
    };

    Ok((axum::http::StatusCode::OK, Json(response)))
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
}

impl IntoResponse for QueryError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            QueryError::Default(e) => (
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