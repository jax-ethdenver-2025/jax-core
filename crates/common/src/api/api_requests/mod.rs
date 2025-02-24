use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

mod hello;

pub use hello::Hello;

/// Defintion of an API request
pub trait ApiRequest {
    /// Has a response type
    type Response: DeserializeOwned;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
}
