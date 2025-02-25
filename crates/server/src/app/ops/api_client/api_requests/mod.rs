mod health;
mod list_blobs;
mod share;

pub use health::{Liveness, Readiness};
pub use list_blobs::{BlobInfo, ListBlobs};
pub use share::Share;

use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

/// Defintion of an API request
pub trait ApiRequest: Send + Sync {
    /// Has a response type
    type Response: DeserializeOwned;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
}
