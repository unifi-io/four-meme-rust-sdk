use thiserror::Error;


#[derive(Debug, Error)]
pub enum FourMemeError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("contract error: {0}")]
    Contract(String),
    #[error("abi error: {0}")]
    Abi(String),
    #[error("other: {0}")]
    Other(String),
}
