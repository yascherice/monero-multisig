use thiserror::Error;

/// Top-level error type for the multisig wallet tool.
#[derive(Error, Debug)]
pub enum MultisigError {
    #[error("wallet error: {0}")]
    Wallet(#[from] WalletError),

    #[error("transaction error: {0}")]
    Transaction(#[from] TransactionError),

    #[error("configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Errors specific to wallet operations.
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("invalid multisig parameters: {0}")]
    InvalidParams(String),

    #[error("wallet not found at {0}")]
    NotFound(String),

    #[error("wallet already exists at {0}")]
    AlreadyExists(String),

    #[error("key exchange failed: {0}")]
    KeyExchangeFailed(String),

    #[error("wallet is not ready — complete key exchange first")]
    NotReady,
}

/// Errors specific to transaction operations.
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("insufficient balance: need {need} but have {have}")]
    InsufficientBalance { need: u64, have: u64 },

    #[error("invalid destination address: {0}")]
    InvalidAddress(String),

    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("not enough signatures: have {have}, need {need}")]
    InsufficientSignatures { have: u32, need: u32 },

    #[error("transaction rejected by daemon: {0}")]
    Rejected(String),
}
