use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;

use crate::api::api_requests::ApiRequest;

pub struct Hello;

#[derive(Debug, Deserialize)]
pub struct HelloResponse {
    message: String,
}

impl HelloResponse {
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl ApiRequest for Hello {
    type Response = HelloResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v0/hello").unwrap();
        client.get(full_url)
    }
}
