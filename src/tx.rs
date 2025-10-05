use rust_decimal::Decimal;

use crate::{ClientId, TransactionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    Resolve {
        client: ClientId,
        tx: TransactionId,
    },
    Chargeback {
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
            Transaction::Resolve { tx, .. } => *tx,
            Transaction::Chargeback { tx, .. } => *tx,
        }
    }

    pub fn client_id(&self) -> ClientId {
        match self {
            Transaction::Deposit { client, .. } => *client,
            Transaction::Withdrawal { client, .. } => *client,
            Transaction::Dispute { client, .. } => *client,
            Transaction::Resolve { client, .. } => *client,
            Transaction::Chargeback { client, .. } => *client,
        }
    }
}
