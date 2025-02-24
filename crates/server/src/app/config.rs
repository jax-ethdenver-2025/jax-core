use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use dotenvy::dotenv;

#[derive(Debug)]
pub struct Config {
    // Listen address
    listen_addr: SocketAddr,

    // Logging Level
    log_level: tracing::Level,
}

impl Config {
    pub fn from_env() -> Result<Config, ConfigError> {
        if dotenv().is_err() {
            tracing::warn!("No .env file found");
        }

        let listen_addr_str = match env::var("LISTEN_ADDR") {
            Ok(addr) => addr,
            Err(_e) => {
                tracing::warn!("No LISTEN_ADDR found in .env. Using default");
                "0.0.0.0:8080".to_string()
            }
        };
        let listen_addr = listen_addr_str.parse()?;

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
            listen_addr,
            log_level,
        })
    }

    pub fn log_level(&self) -> &tracing::Level {
        &self.log_level
    }

    pub fn listen_addr(&self) -> &SocketAddr {
        &self.listen_addr
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing Env: {0}")]
    Env(#[from] env::VarError),
    #[error("Invalid Socket Address: {0}")]
    ListenAddr(#[from] std::net::AddrParseError),
}
