use async_trait::async_trait;

use crate::app::{args::Op, config::ConfigError, Config};
use crate::server::spawn;

#[derive(Debug, clap::Args, Clone)]
pub struct Serve;

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

#[async_trait]
impl Op for Serve {
    type Error = ServeError;
    type Output = ();

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        let config = Config::from_env_or_disk()?;
        spawn(&config).await;
        Ok(())
    }
}
