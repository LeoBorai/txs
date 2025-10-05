use rust_decimal::Decimal;

use crate::{ClientId, TransactionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomesticTransaction {
    pub amount: Decimal,
    pub client: ClientId,
    pub tx: TransactionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SupportTransaction {
    pub client: ClientId,
    pub tx: TransactionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Transaction {
    Deposit(DomesticTransaction),
    Withdrawal(DomesticTransaction),
    Dispute(SupportTransaction),
    Resolve(SupportTransaction),
    Chargeback(SupportTransaction),
}

impl Transaction {
    pub fn id(&self) -> TransactionId {
        match self {
            Transaction::Deposit(DomesticTransaction { tx, .. }) => *tx,
            Transaction::Withdrawal(DomesticTransaction { tx, .. }) => *tx,
            Transaction::Dispute(SupportTransaction { tx, .. }) => *tx,
            Transaction::Resolve(SupportTransaction { tx, .. }) => *tx,
            Transaction::Chargeback(SupportTransaction { tx, .. }) => *tx,
        }
    }

    pub fn client_id(&self) -> ClientId {
        match self {
            Transaction::Deposit(DomesticTransaction { client, .. }) => *client,
            Transaction::Withdrawal(DomesticTransaction { client, .. }) => *client,
            Transaction::Dispute(SupportTransaction { client, .. }) => *client,
            Transaction::Resolve(SupportTransaction { client, .. }) => *client,
            Transaction::Chargeback(SupportTransaction { client, .. }) => *client,
        }
    }
}
