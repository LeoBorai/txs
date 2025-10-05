use rust_decimal::Decimal;

use crate::error::{Error, Result};
use crate::{ClientId, TransactionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub client: ClientId,
    pub r#type: TransactionType,
    pub id: TransactionId,
    pub amount: Option<Decimal>,
}

impl Transaction {
    pub fn amount(&self) -> Result<Decimal> {
        self.amount
            .ok_or(Error::DomesticTransactionMissingAmount { tx: self.clone() })
    }
}
