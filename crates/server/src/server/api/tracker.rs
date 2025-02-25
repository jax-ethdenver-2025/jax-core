use crate::server::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use iroh::NodeId;
use iroh_blobs::{BlobFormat, Hash};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnounceRequest {
    hash: String,
    format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnounceResponse {
    success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoverResponse {
    nodes: Vec<String>,
}

// Announce content
pub async fn announce(
    State(state): State<AppState>,
    Json(req): Json<AnnounceRequest>,
) -> Json<AnnounceResponse> {
    // Parse hash and format
    let hash = match Hash::from_str(&req.hash) {
        Ok(h) => h,
        Err(_) => return Json(AnnounceResponse { success: false }),
    };
    
    let format = match req.format.as_str() {
        "raw" => BlobFormat::Raw,
        "car" => BlobFormat::Car,
        _ => return Json(AnnounceResponse { success: false }),
    };
    
    // Announce the content
    match state.tracker_service().announce_content(hash, format).await {
        Ok(_) => Json(AnnounceResponse { success: true }),
        Err(_) => Json(AnnounceResponse { success: false }),
    }
}

// Discover content
pub async fn discover(
    State(state): State<AppState>,
    Path((hash_str, format_str)): Path<(String, String)>,
) -> Json<DiscoverResponse> {
    // Parse hash and format
    let hash = match Hash::from_str(&hash_str) {
        Ok(h) => h,
        Err(_) => return Json(DiscoverResponse { nodes: vec![] }),
    };
    
    let format = match format_str.as_str() {
        "raw" => BlobFormat::Raw,
        "car" => BlobFormat::Car,
        _ => return Json(DiscoverResponse { nodes: vec![] }),
    };
    
    // Find nodes with the content
    match state.tracker_service().discover_content(hash, format).await {
        Ok(nodes) => {
            let node_strs = nodes.into_iter()
                .map(|n| n.to_string())
                .collect();
            Json(DiscoverResponse { nodes: node_strs })
        },
        Err(_) => Json(DiscoverResponse { nodes: vec![] }),
    }
} 