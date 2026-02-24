use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("OMP subprocess error: {0}")]
    OmpSubprocess(String),

    #[error("OMP process exited: {0}")]
    OmpExited(std::process::ExitStatus),

    #[error("Discord error: {0}")]
    Discord(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Unknown RPC command: {0}")]
    UnknownRpcCommand(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Correlation ID not found: {0}")]
    CorrelationNotFound(String),

    #[error("Timeout waiting for response: {0}")]
    Timeout(String),
}
