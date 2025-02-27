mod health;
mod list;
mod share;
mod query;
mod create_pool;

pub use health::{Liveness, Readiness};
pub use create_pool::{CreatePool, CreatePoolResponse};
pub use list::{List, ListResponse};
pub use share::Share;
pub use query::Query;

use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

/// Defintion of an API request
pub trait ApiRequest: Send + Sync {
    /// Has a response type
    type Response: DeserializeOwned;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
}