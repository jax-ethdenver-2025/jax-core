use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;

use super::ApiRequest;

#[derive(Debug, Clone)]
pub struct ListBlobs;

#[derive(Debug, Clone, Deserialize)]
pub struct BlobInfo {
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListBlobsResponse {
    pub blobs: Vec<BlobInfo>,
}

impl ApiRequest for ListBlobs {
    type Response = ListBlobsResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let url = base_url.join("api/v0/blobs").expect("Failed to join URL");
        client.get(url)
    }
}
