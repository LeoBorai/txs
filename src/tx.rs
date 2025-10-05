use rust_decimal::Decimal;

use crate::{ClientId, TransactionId};

#[derive(Debug, Clone)]
pub enum Transaction {
    Deposit {
        amount: Decimal,
        client: ClientId,
        tx: TransactionId,
    },
    Withdrawal {
        amount: Decimal,
        client: ClientId,
        tx: TransactionId,
    },
    Dispute {
        client: ClientId,
        tx: TransactionId,
    },
}

impl Transaction {
    pub fn id(&self) -> TransactionId {
        match self {
            Transaction::Deposit { tx, .. } => *tx,
            Transaction::Withdrawal { tx, .. } => *tx,
            Transaction::Dispute { tx, .. } => *tx,
        }
    }
}
