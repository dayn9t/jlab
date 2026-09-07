use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Export error: {0}")]
    Export(String),
}

pub type Result<T> = std::result::Result<T, Error>;
