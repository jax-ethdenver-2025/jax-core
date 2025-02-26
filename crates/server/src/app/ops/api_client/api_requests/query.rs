use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use iroh::NodeId;
use iroh_blobs::Hash;

use super::ApiRequest;

#[derive(Debug, Clone, Serialize)]
pub struct Query {
    pub hash: Hash,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryResponse {
    pub nodes: Vec<NodeId>,
    pub message: String,
}

impl ApiRequest for Query {
    type Response = QueryResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let url = base_url.join("api/v0/query").expect("Failed to join URL");
        client.post(url).json(&self)
    }
} 