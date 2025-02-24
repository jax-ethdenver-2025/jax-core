use super::config::Config;

#[derive(Clone)]
pub struct AppState {
    // TODO: add stuff here
}

impl AppState {
    pub async fn from_config(_config: &Config) -> Result<Self, AppStateSetupError> {
        Ok(Self {})
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateSetupError {}
