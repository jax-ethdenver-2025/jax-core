use async_trait::async_trait;

use crate::{AppState, Op};

#[derive(Debug, clap::Args, Clone)]
pub struct Hello {}

#[derive(Debug, thiserror::Error)]
pub enum HelloError {
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
    #[error("app state error: {0}")]
    AppState(#[from] crate::state::AppStateSetupError),
    #[error("api error: {0}")]
    ApiError(#[from] jax_common::error::ApiError),
}

#[async_trait]
impl Op for Hello {
    type Error = HelloError;
    type Output = String;

    async fn execute(&self, state: &AppState) -> Result<Self::Output, Self::Error> {
        let client = state.client()?;
        let hello = client.call(jax_common::prelude::Hello {}).await?;
        Ok(hello.message().to_string())
    }
}
