use thiserror::Error;

use crate::tx::Transaction;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Account not found for client: {tx:?}")]
    AccountNotFound { tx: Transaction },
    #[error("Insufficient funds to perform transaction: {tx:?}")]
    InsufficientFunds { tx: Transaction },
    #[error("Transaction not found: {tx:?}")]
    TransactionNotFound { tx: Transaction },
    #[error("Dispute Transaction not found: {tx:?}. No dispute in progress.")]
    DisputeTxNotFound { tx: Transaction },
    #[error("Account locked. Transaction cannot be pocessed: {tx:?}")]
    LockedAccount { tx: Transaction },
    #[error("Account has inconsistent held funds: {tx:?}")]
    IncosistentHeldFunds { tx: Transaction },
    #[error("Domestic transaction is missing amount invalid: {tx:?}")]
    DomesticTransactionMissingAmount { tx: Transaction },
}
