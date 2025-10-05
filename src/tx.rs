use rust_decimal::Decimal;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::{ClientId, TransactionId};

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub client: ClientId,
    pub r#type: TransactionType,
    #[serde(rename = "tx")]
    pub id: TransactionId,
    #[serde(default)]
    pub amount: Option<Decimal>,
}

impl Transaction {
    pub fn amount(&self) -> Result<Decimal> {
        self.amount
            .ok_or(Error::DomesticTransactionMissingAmount { tx: self.clone() })
    }
}
