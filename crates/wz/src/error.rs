use thiserror::Error;

#[derive(Debug, Error)]
pub enum WzError {
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("parse error: {0}")]
    WzError(#[from] wz_reader::node::Error),
    #[error("lock poisoned")]
    LockPoisoned,
    #[error("type mismatch: expected {0}")]
    TypeMismatch(&'static str),
    #[error("value error: {0}")]
    ValueError(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("value error: {0}")]
    ValueError(String),
}
