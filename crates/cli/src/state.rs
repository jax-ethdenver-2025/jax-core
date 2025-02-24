use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use jax_common::prelude::*;

use crate::args::Command;

use super::Args;

pub const DEFAULT_HOME_ENV: &str = "HOME";
pub const DEFAULT_XDG_CONFIG_DIR: &str = ".config";
pub const DEFAULT_XDG_CONFIG_DIR_NAME: &str = "jax";
pub const DEFAULT_REMOTE: &str = "http://localhost:8080";
pub const DEFAULT_CONFIG_NAME: &str = "jax.conf";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDiskConfig {
    pub remote: Url,
}

pub struct AppState {
    pub on_disk_config: OnDiskConfig,
}

impl AppState {
    fn find_xdg_config_dir() -> PathBuf {
        let home_dir_env = std::env::var(DEFAULT_HOME_ENV).expect("HOME is not set");
        let home_dir = PathBuf::from(home_dir_env);
        let xdg_dir = home_dir.join(DEFAULT_XDG_CONFIG_DIR);
        xdg_dir.join(DEFAULT_XDG_CONFIG_DIR_NAME)
    }

    pub fn try_from(args: &Args) -> Result<Self, AppStateSetupError> {
        let cmd = &args.command;
        if matches!(cmd, Command::Init(_)) {
            Self::init_on_disk_config()?;
        }

        let on_disk_config = AppState::load_on_disk_config()?;
        Ok(Self { on_disk_config })
    }

    pub fn client(&self) -> Result<ApiClient, AppStateSetupError> {
        let remote = self.on_disk_config.remote.clone();
        let client = ApiClient::new(remote.as_str())?;
        Ok(client)
    }

    pub fn init_on_disk_config() -> Result<(), AppStateSetupError> {
        let path = Self::find_xdg_config_dir();
        if path.exists() {
            return Err(AppStateSetupError::ConfigAlreadyExists);
        }
        std::fs::create_dir_all(path.clone())
            .map_err(|e| AppStateSetupError::Io(e, path.clone()))?;

        let config_path = path.join(PathBuf::from(DEFAULT_CONFIG_NAME));
        let on_disk_config = OnDiskConfig {
            remote: DEFAULT_REMOTE.parse().unwrap(),
        };
        // Write everything to disk
        let config_json = serde_json::to_string(&on_disk_config)?;
        std::fs::write(&config_path, config_json)
            .map_err(|e| AppStateSetupError::Io(e, config_path))?;
        Ok(())
    }

    pub fn load_on_disk_config() -> Result<OnDiskConfig, AppStateSetupError> {
        let path = Self::find_xdg_config_dir();
        if !path.exists() {
            return Err(AppStateSetupError::MissingConfig);
        }

        let config_path = path.join(PathBuf::from(DEFAULT_CONFIG_NAME));

        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AppStateSetupError::Io(e, config_path))?;
        let config: OnDiskConfig = serde_json::from_str(&config_str)?;

        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateSetupError {
    #[error("default: {0}")]
    Default(#[from] anyhow::Error),
    #[error("io: {0:?} path: {1:?}")]
    Io(std::io::Error, PathBuf),
    #[error("invalid config: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("config already exists")]
    ConfigAlreadyExists,
    #[error("missing config")]
    MissingConfig,
    #[error("api error: {0}")]
    ApiError(#[from] jax_common::error::ApiError),
}
