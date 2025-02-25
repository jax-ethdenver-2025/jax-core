use async_trait::async_trait;
use iroh::NodeId;

use crate::app::{
    args::Op,
    config::{ConfigError, OnDiskConfig},
    Config,
};

#[derive(Debug, clap::Args, Clone)]
pub struct Init {
    #[arg(short, long, default_value_t = false)]
    pub overwrite: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

#[async_trait]
impl Op for Init {
    type Error = InitError;
    type Output = NodeId;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        OnDiskConfig::init(self.overwrite)?;
        let config = Config::from_env_or_disk()?;
        let key = config.key()?;
        let node_id = key.public();
        Ok(node_id)
    }
}
