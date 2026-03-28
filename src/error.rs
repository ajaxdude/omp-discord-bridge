use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Discord error: {0}")]
    Discord(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("MCP server error: {0}")]
    Mcp(String),
}
