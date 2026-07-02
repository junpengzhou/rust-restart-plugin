use std::path::PathBuf;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("command `{program}` failed with exit code {code}: {stderr}")]
    CommandFailed {
        program: String,
        code: i32,
        stderr: String,
    },

    #[error("failed to run command `{program}`: {source}")]
    CommandIo {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse JSON from `{path}`: {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse Docker inspect JSON: {0}")]
    DockerJson(#[from] serde_json::Error),

    #[error("container `{0}` failed to start")]
    RestartFailed(String),

    #[error("unable to find host port mapped to container port {port} for {module}")]
    MissingPortMapping { module: String, port: String },

    #[error("startup health check is unknown")]
    HealthCheckUnknown,

    #[error("notification configuration is invalid: {0}")]
    InvalidConfig(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
