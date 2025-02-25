use async_trait::async_trait;
use std::fmt;

use super::api_client::{api_requests, ApiClient, ApiError};
use crate::app::args::Op;
use crate::app::{Config, ConfigError};

#[derive(Debug, clap::Args, Clone)]
pub struct ListBlobs;

#[derive(Debug, thiserror::Error)]
pub enum ListBlobsError {
    #[error("api error: {0}")]
    Api(#[from] ApiError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

#[derive(Debug)]
pub struct ListBlobsOutput {
    blobs: Vec<api_requests::BlobInfo>,
}

impl fmt::Display for ListBlobsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.blobs.is_empty() {
            writeln!(f, "No blobs found")?;
            return Ok(());
        }

        writeln!(f, "Available blobs:")?;
        for blob in &self.blobs {
            writeln!(f, "Hash: {}", blob.hash)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Op for ListBlobs {
    type Error = ListBlobsError;
    type Output = ListBlobsOutput;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        let config = Config::from_env_or_disk()?;
        let client = ApiClient::new(config.remote_url().as_ref())?;

        // Create request
        let request = api_requests::ListBlobs;

        // Call the API endpoint
        let response = client.call(request).await?;

        Ok(ListBlobsOutput {
            blobs: response.blobs,
        })
    }
}
