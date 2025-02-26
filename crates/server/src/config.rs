use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use alloy::signers::local::{LocalWallet, PrivateKeySigner};
use dotenvy::dotenv;
use iroh::SecretKey;
use iroh_blobs::store::fs::{BatchOptions, InlineOptions, Options, PathOptions, Store};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use url::Url;

pub const DEFAULT_HOME_ENV: &str = "HOME";
pub const DEFAULT_XDG_CONFIG_DIR: &str = ".config";
pub const DEFAULT_XDG_CONFIG_DIR_NAME: &str = "jax";
pub const DEFAULT_CONFIG_NAME: &str = "jax.conf";
pub const CONFIG_PATH_ENV: &str = "JAX_CONFIG_PATH";

#[derive(Serialize, Deserialize)]
pub struct BlobsOptions {
    pub path_data: PathBuf,
    pub path_temp: PathBuf,
    pub inline_max_bytes: Option<u64>,
}

impl BlobsOptions {
    pub fn to_options(&self) -> Options {
        Options {
            path: PathOptions {
                data_path: self.path_data.clone(),
                temp_path: self.path_temp.clone(),
            },
            inline: InlineOptions::NO_INLINE,
            batch: BatchOptions {
                max_read_batch: 10,
                max_read_duration: std::time::Duration::from_secs(1),
                max_write_batch: 10,
                max_write_duration: std::time::Duration::from_secs(1),
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OnDiskConfig {
    pub remote_listen_addr: SocketAddr,
    pub endpoint_listen_addr: SocketAddr,
    pub blobs_path: PathBuf,
    pub blobs_option: BlobsOptions,
    pub iroh_key_file_path: PathBuf,
    pub eth_key_file_path: PathBuf,
}

impl Default for OnDiskConfig {
    fn default() -> Self {
        Self {
            remote_listen_addr: "127.0.0.1:8080".parse().unwrap(),
            endpoint_listen_addr: "0.0.0.0:3001".parse().unwrap(),
            // relative to xdg config dir
            blobs_path: PathBuf::from("blobs"),
            blobs_option: BlobsOptions {
                path_data: PathBuf::from("data"),
                path_temp: PathBuf::from("temp"),
                inline_max_bytes: None,
            },
            // relative to xdg config dir
            iroh_key_file_path: PathBuf::from("iroh_key.bin"),
            eth_key_file_path: PathBuf::from("eth_key.bin"),
        }
    }
}

impl OnDiskConfig {
    pub fn config_path() -> PathBuf {
        let path = Self::find_config_dir();
        path.join(PathBuf::from(DEFAULT_CONFIG_NAME))
    }

    pub fn remote_listen_addr(&self) -> SocketAddr {
        self.remote_listen_addr
    }

    pub fn endpoint_listen_addr(&self) -> SocketAddr {
        self.endpoint_listen_addr
    }

    pub fn iroh_key_file_path(&self) -> PathBuf {
        let path = Self::find_config_dir();
        path.join(self.iroh_key_file_path.clone())
    }

    pub fn eth_key_file_path(&self) -> PathBuf {
        let path = Self::find_config_dir();
        path.join(self.eth_key_file_path.clone())
    }

    pub fn blobs_path(&self) -> PathBuf {
        let path = Self::find_config_dir();
        path.join(self.blobs_path.clone())
    }

    pub fn find_config_dir() -> PathBuf {
        match std::env::var(CONFIG_PATH_ENV) {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                let home_dir_env = std::env::var(DEFAULT_HOME_ENV).expect("HOME is not set");
                let home_dir = PathBuf::from(home_dir_env);
                let xdg_dir = home_dir.join(DEFAULT_XDG_CONFIG_DIR);
                xdg_dir.join(DEFAULT_XDG_CONFIG_DIR_NAME)
            }
        }
    }

    pub fn init(overwrite: bool) -> Result<(), ConfigError> {
        let path = Self::find_config_dir();
        if path.exists() {
            if overwrite {
                std::fs::remove_dir_all(&path).map_err(|e| ConfigError::Io(e, path.clone()))?;
            } else {
                return Err(ConfigError::ConfigAlreadyExists);
            }
        }
        std::fs::create_dir_all(path.clone()).map_err(|e| ConfigError::Io(e, path.clone()))?;

        let on_disk_config = OnDiskConfig::default();

        let mut rng = OsRng;
        let iroh_secret_key = SecretKey::generate(&mut rng);

        let config_json = serde_json::to_string(&on_disk_config)?;
        let iroh_key_bytes = iroh_secret_key.to_bytes();
        let eth_key = PrivateKeySigner::random();
        let eth_key_bytes = eth_key.to_bytes();

        let blobs_path = on_disk_config.blobs_path();
        let iroh_key_path = on_disk_config.iroh_key_file_path();
        let eth_key_path = on_disk_config.eth_key_file_path();
        let config_path = Self::config_path();

        std::fs::create_dir_all(blobs_path.clone())
            .map_err(|e| ConfigError::Io(e, blobs_path.clone()))?;
        let _store = Store::new(blobs_path, on_disk_config.blobs_option());

        std::fs::write(&config_path, config_json).map_err(|e| ConfigError::Io(e, config_path))?;

        std::fs::write(&iroh_key_path, iroh_key_bytes)
            .map_err(|e| ConfigError::Io(e, iroh_key_path))?;
        std::fs::write(&eth_key_path, eth_key_bytes)
            .map_err(|e| ConfigError::Io(e, eth_key_path))?;
        Ok(())
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::find_config_dir();
        if !path.exists() {
            return Err(ConfigError::MissingConfig);
        }

        let config_path = Self::config_path();
        let config_str =
            std::fs::read_to_string(&config_path).map_err(|e| ConfigError::Io(e, config_path))?;
        let config: OnDiskConfig = serde_json::from_str(&config_str)?;

        Ok(config)
    }

    pub fn blobs_option(&self) -> Options {
        self.blobs_option.to_options()
    }
}

#[derive(Debug)]
pub struct Config {
    remote_listen_addr: SocketAddr,
    endpoint_listen_addr: SocketAddr,
    iroh_key_file_path: PathBuf,
    eth_key_file_path: PathBuf,
    blobs_path: PathBuf,

    // Logging Level
    log_level: tracing::Level,
}

impl Config {
    pub fn from_env_or_disk() -> Result<Config, ConfigError> {
        if dotenv().is_err() {
            tracing::warn!("No .env file found");
        }

        let on_disk_config = OnDiskConfig::load()?;

        let remote_listen_addr = match env::var("REMOTE_LISTEN_ADDR") {
            Ok(addr) => addr.parse()?,
            Err(_e) => on_disk_config.remote_listen_addr(),
        };

        let endpoint_listen_addr = match env::var("ENDPOINT_LISTEN_ADDR") {
            Ok(addr) => addr.parse()?,
            Err(_e) => on_disk_config.endpoint_listen_addr(),
        };

        let log_level_str = match env::var("LOG_LEVEL") {
            Ok(level) => level,
            Err(_e) => {
                tracing::warn!("No LOG_LEVEL found in .env. Using default");
                "info".to_string()
            }
        };
        let log_level = match tracing::Level::from_str(&log_level_str) {
            Ok(level) => level,
            Err(_e) => {
                tracing::warn!("Invalid LOG_LEVEL found in .env. Using default");
                tracing::Level::INFO
            }
        };

        Ok(Config {
            remote_listen_addr,
            endpoint_listen_addr,
            iroh_key_file_path: on_disk_config.iroh_key_file_path(),
            eth_key_file_path: on_disk_config.eth_key_file_path(),
            blobs_path: on_disk_config.blobs_path(),
            log_level,
        })
    }

    pub fn log_level(&self) -> &tracing::Level {
        &self.log_level
    }

    pub fn remote_listen_addr(&self) -> &SocketAddr {
        &self.remote_listen_addr
    }

    pub fn remote_url(&self) -> Url {
        // NOTE (amiller68): for now this is local only http api
        let scheme = "http";
        let host = self.remote_listen_addr.ip().to_string();
        let port = self.remote_listen_addr.port();

        Url::parse(&format!("{}://{}:{}", scheme, host, port))
            .expect("Failed to construct remote URL")
    }

    pub fn endpoint_listen_addr(&self) -> &SocketAddr {
        &self.endpoint_listen_addr
    }

    pub fn iroh_secret_key(&self) -> Result<SecretKey, ConfigError> {
        let key_bytes = std::fs::read(&self.iroh_key_file_path)
            .map_err(|e| ConfigError::Io(e, self.iroh_key_file_path.clone()))?;

        // Convert Vec<u8> to [u8; 32]
        if key_bytes.len() != 32 {
            return Err(ConfigError::InvalidKeyBytes);
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&key_bytes);

        Ok(SecretKey::from_bytes(&array))
    }

    pub fn eth_signer(&self) -> Result<PrivateKeySigner, ConfigError> {
        let key_bytes = std::fs::read(&self.eth_key_file_path)
            .map_err(|e| ConfigError::Io(e, self.eth_key_file_path.clone()))?;

        let mut array = [0u8; 32];
        array.copy_from_slice(&key_bytes);

        Ok(PrivateKeySigner::from_bytes(&array.into())
            .expect("Failed to convert key bytes to PrivateKeySigner"))
    }

    pub fn blobs_path(&self) -> &PathBuf {
        &self.blobs_path
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
    #[error("io: {0:?} path: {1:?}")]
    Io(std::io::Error, PathBuf),
    #[error("invalid config: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Missing Env: {0}")]
    Env(#[from] env::VarError),
    #[error("Invalid Socket Address: {0}")]
    ListenAddr(#[from] std::net::AddrParseError),
    #[error("missing config")]
    MissingConfig,
    #[error("invalid key bytes")]
    InvalidKeyBytes,
    #[error("config already exists")]
    ConfigAlreadyExists,
}
