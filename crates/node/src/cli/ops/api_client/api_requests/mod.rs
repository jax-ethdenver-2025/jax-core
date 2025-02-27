mod health;
mod list;
mod share;
mod query;
mod create_pool;
mod probe;
mod pools;

pub use health::{Liveness, Readiness};
pub use create_pool::{CreatePool, CreatePoolResponse};
pub use list::{List, ListResponse};
pub use share::Share;
pub use query::Query;
pub use probe::{Probe, ProbeStats};
pub use pools::{Pools, PoolsResponse};

use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

/// Defintion of an API request
pub trait ApiRequest: Send + Sync {
    /// Has a response type
    type Response: DeserializeOwned;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
}