use async_trait::async_trait;
use std::path::PathBuf;

use super::api_client::{api_requests, ApiClient, ApiError};
use crate::app::args::Op as AppOp;
use crate::app::{Config, ConfigError};

#[derive(Debug, clap::Args, Clone)]
pub struct Share {
    /// Path to the file to share
    #[arg(short, long)]
    pub path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("api error: {0}")]
    Api(#[from] ApiError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
impl AppOp for Share {
    type Error = ShareError;
    type Output = String;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        let config = Config::from_env_or_disk()?;
        let client = ApiClient::new(config.remote_url().as_ref())?;

        // Get absolute path for the file
        let abs_path = std::path::absolute(&self.path)?;

        // Create request
        let request = api_requests::Share {
            path: abs_path.to_string_lossy().to_string(),
        };

        // Call the API endpoint
        let response = client.call(request).await?;

        Ok(format!(
            "{}\nShare ticket: {}\nHash: {}",
            response.message, response.ticket, response.hash
        ))
    }
}
