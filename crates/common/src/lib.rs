#[allow(unused_imports)]
#[allow(dead_code)]
mod api;

pub mod prelude {
    pub use crate::api::{
        api_requests::{ApiRequest, Hello},
        ApiClient,
    };
}

pub mod error {
    pub use crate::api::ApiError;
}
