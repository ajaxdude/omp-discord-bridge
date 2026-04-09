use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Discord bot token
    pub discord_token: String,
    /// Discord command prefix
    #[serde(default = "default_discord_prefix")]
    pub discord_prefix: String,
    /// OMP executable path
    #[serde(default = "default_omp_path")]
    pub omp_path: String,
    /// Working directory for OMP subprocess.
    /// Defaults to $HOME so coding sessions start in the user's home directory.
    /// Override with OMP_WORK_DIR in .env to point at a specific project root.
    #[serde(default = "default_omp_work_dir")]
    pub omp_work_dir: String,
    /// Path to config.yaml for model aliases.
    /// Defaults to $HOME/.config/omp-discord-bridge/config.yaml.
    #[serde(default = "default_aliases_config_path")]
    pub aliases_config_path: String,
}

fn default_discord_prefix() -> String {
    "!".to_string()
}

fn default_omp_path() -> String {
    "omp".to_string()
}

fn default_omp_work_dir() -> String {
    env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

fn default_aliases_config_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{home}/.config/omp-discord-bridge/config.yaml")
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|_| Error::MissingEnvVar("DISCORD_TOKEN".to_string()))?;

        let discord_prefix = env::var("DISCORD_PREFIX")
            .unwrap_or_else(|_| "!".to_string());

        let omp_path = env::var("OMP_PATH")
            .unwrap_or_else(|_| "omp".to_string())
            .split('#').next().unwrap_or("omp").trim().to_string();

        let omp_work_dir = env::var("OMP_WORK_DIR")
            .unwrap_or_else(|_| default_omp_work_dir());


        let aliases_config_path = env::var("BRIDGE_CONFIG")
            .unwrap_or_else(|_| default_aliases_config_path());
        Ok(Self {
            discord_token,
            discord_prefix,
            omp_path,
            omp_work_dir,
            aliases_config_path,
        })
    }

    #[allow(dead_code)]
    /// Load configuration from a file
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = serde_json::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config file: {}", e)))?;

        let discord_token = env::var("DISCORD_TOKEN")
            .unwrap_or(config.discord_token);

        let discord_prefix = env::var("DISCORD_PREFIX")
            .unwrap_or(config.discord_prefix);

        let omp_path = env::var("OMP_PATH")
            .unwrap_or(config.omp_path)
            .split('#').next().unwrap_or("omp").trim().to_string();

        let omp_work_dir = env::var("OMP_WORK_DIR")
            .unwrap_or(config.omp_work_dir);

        let aliases_config_path = env::var("BRIDGE_CONFIG")
            .unwrap_or(config.aliases_config_path);

        Ok(Self {
            discord_token,
            discord_prefix,
            omp_path,
            omp_work_dir,
            aliases_config_path,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.discord_token.is_empty() {
            return Err(Error::Config("Discord token is empty".to_string()));
        }

        if self.discord_prefix.is_empty() {
            return Err(Error::Config("Discord prefix is empty".to_string()));
        }

        Ok(())
    }
}
