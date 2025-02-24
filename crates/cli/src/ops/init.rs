use async_trait::async_trait;

use crate::{AppState, Op};

#[derive(Debug, clap::Args, Clone)]
pub struct Init {}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
    #[error("app state error: {0}")]
    AppState(#[from] crate::state::AppStateSetupError),
}

#[async_trait]
impl Op for Init {
    type Error = InitError;
    type Output = String;

    async fn execute(&self, _state: &AppState) -> Result<Self::Output, Self::Error> {
        Ok("lol i don't do anything".to_string())
    }
}
