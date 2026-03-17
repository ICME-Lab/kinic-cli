//! Error types for Oracle

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OracleError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Cargo metadata error: {0}")]
    CargoMetadata(#[from] cargo_metadata::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, OracleError>;

impl From<syn::Error> for OracleError {
    fn from(e: syn::Error) -> Self {
        OracleError::Parse(e.to_string())
    }
}

impl From<crate::domain_rust::error::OracleError> for OracleError {
    fn from(e: crate::domain_rust::error::OracleError) -> Self {
        match e {
            crate::domain_rust::error::OracleError::Io(err) => OracleError::Io(err),
            crate::domain_rust::error::OracleError::Parse(msg) => OracleError::Parse(msg),
            crate::domain_rust::error::OracleError::Config(msg) => OracleError::Config(msg),
            crate::domain_rust::error::OracleError::Network(err) => OracleError::Network(err),
            crate::domain_rust::error::OracleError::CargoMetadata(err) => {
                OracleError::CargoMetadata(err)
            }
            crate::domain_rust::error::OracleError::Yaml(err) => OracleError::Yaml(err),
            crate::domain_rust::error::OracleError::Analysis(msg) => OracleError::Analysis(msg),
            crate::domain_rust::error::OracleError::Other(msg) => OracleError::Other(msg),
        }
    }
}
