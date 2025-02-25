use async_trait::async_trait;

use super::api_client::{
    requests::{Liveness, Readiness},
    ApiClient, ApiError,
};

use crate::app::args::Op;
use crate::app::{Config, ConfigError};

#[derive(Debug, clap::Args, Clone)]
pub struct Status {}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("api error: {0}")]
    Api(#[from] ApiError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

#[async_trait]
impl Op for Status {
    type Error = StatusError;
    type Output = String;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        let config = Config::from_env_or_disk()?;
        let client = ApiClient::new(config.remote_url().as_ref())?;

        // Check liveness
        let liveness = client.call(Liveness).await?;

        // Check readiness
        let readiness = client.call(Readiness).await?;

        let output = format!(
            "Server Status:\n- Liveness: {}\n- Readiness: {}\n- Node ID: {}",
            liveness.status(),
            readiness.status(),
            readiness.node_id()
        );

        Ok(output)
    }
}
