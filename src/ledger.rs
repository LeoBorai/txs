use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::account::Account;
use crate::tx::Transaction;
use crate::{ClientId, TransactionId};

pub type Result<T> = std::result::Result<T, LedgerError>;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Account not found for client: {tx}")]
    AccountNotFound { tx: TransactionId },
    #[error("Insufficient funds to perform transaction: {tx}")]
    InsufficientFunds { tx: TransactionId },
    #[error("Transaction not found: {tx}")]
    TransactionNotFound { tx: TransactionId },
    #[error("Invalid Transaction for Dispute: {tx}")]
    InvalidTransactionForDispute { tx: TransactionId },
}

pub struct Ledger {
    accounts: HashMap<ClientId, Account>,
    tx_log: HashSet<Transaction>,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            tx_log: HashSet::new(),
        }
    }

    pub fn get_account(&self, client_id: &ClientId) -> Option<&Account> {
        self.accounts.get(client_id)
    }

    pub fn get_tx(&self, tx: &Transaction) -> Option<&Transaction> {
        self.tx_log.iter().find(|t| *t == tx)
    }

    pub fn find_tx<P>(&self, p: P) -> Option<&Transaction>
    where
        P: Fn(&&Transaction) -> bool,
    {
        self.tx_log.iter().find(p)
    }

    pub fn accounts_iter(&self) -> impl Iterator<Item = (&ClientId, &Account)> {
        self.accounts.iter()
    }

    pub fn tx_log_iter(&self) -> impl Iterator<Item = &Transaction> {
        self.tx_log.iter()
    }

    pub fn process_tx(&mut self, tx: Transaction) -> Result<()> {
        match tx {
            Transaction::Deposit { client, amount, .. } => {
                let account = self.accounts.entry(client).or_default();

                account.available += amount;
                account.total += amount;

                self.tx_log.insert(tx);

                Ok(())
            }
            Transaction::Withdrawal { client, amount, .. } => {
                let Some(account) = self.accounts.get_mut(&client) else {
                    return Err(LedgerError::AccountNotFound { tx: tx.id() });
                };

                if account.available >= amount {
                    account.available -= amount;
                    account.total -= amount;

                    self.tx_log.insert(tx);

                    return Ok(());
                }

                Err(LedgerError::InsufficientFunds { tx: tx.id() })
            }
            Transaction::Dispute { client, tx: tx_id } => {
                // use `cloned` to avoid lifetime issues with mutable borrow
                let Some(target_tx) = self
                    .find_tx(|tx| {
                        tx.id() == tx_id
                            && tx.client_id() == client
                            && matches!(
                                tx,
                                Transaction::Deposit { .. } | Transaction::Withdrawal { .. }
                            )
                    })
                    .cloned()
                else {
                    return Err(LedgerError::TransactionNotFound { tx: tx_id });
                };

                let Some(account) = self.accounts.get_mut(&client) else {
                    return Err(LedgerError::AccountNotFound { tx: tx_id });
                };

                let amount_disputed = match target_tx {
                    Transaction::Deposit { amount, .. } => amount,
                    Transaction::Withdrawal { amount, .. } => amount,
                    _ => {
                        return Err(LedgerError::InvalidTransactionForDispute { tx: tx_id });
                    }
                };

                if account.available >= amount_disputed {
                    account.available -= amount_disputed;
                    account.held += amount_disputed;
                } else {
                    return Err(LedgerError::InsufficientFunds { tx: tx_id });
                }

                self.tx_log.insert(tx);

                Ok(())
            }
            _ => {
                todo!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn ledger_is_empty_by_default() {
        let ledger = Ledger::new();
        assert!(ledger.accounts.is_empty());
    }

    #[test]
    fn process_tx_deposit() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction::Deposit {
            amount: dec!(100.0),
            client: 1,
            tx: 1,
        })?;

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(100.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_withdrawal_handles_insufficient_funds() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction::Deposit {
            amount: dec!(2.0),
            client: 2,
            tx: 1,
        })?;

        let result = ledger.process_tx(Transaction::Withdrawal {
            amount: dec!(3.0),
            client: 2,
            tx: 2,
        });

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(LedgerError::InsufficientFunds { tx: 2 })
        ));

        let account = ledger
            .get_account(&2)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(2.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_two_accounts() -> Result<()> {
        let mut ledger = Ledger::new();

        let _ = ledger.process_tx(Transaction::Deposit {
            amount: dec!(1.0),
            client: 1,
            tx: 1,
        });

        let _ = ledger.process_tx(Transaction::Deposit {
            amount: dec!(2.0),
            client: 2,
            tx: 2,
        });

        let _ = ledger.process_tx(Transaction::Deposit {
            amount: dec!(2.0),
            client: 1,
            tx: 3,
        });

        let _ = ledger.process_tx(Transaction::Withdrawal {
            amount: dec!(1.5),
            client: 1,
            tx: 4,
        });

        let _ = ledger.process_tx(Transaction::Withdrawal {
            amount: dec!(3.0),
            client: 2,
            tx: 5,
        });

        let mut accounts = ledger.accounts_iter().collect::<Vec<_>>();
        accounts.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(b_id));

        assert_eq!(
            accounts[0],
            (
                &1,
                &Account {
                    available: dec!(1.5),
                    held: dec!(0.0),
                    locked: false,
                    total: dec!(1.5),
                }
            )
        );

        assert_eq!(
            accounts[1],
            (
                &2,
                &Account {
                    available: dec!(2.0),
                    held: dec!(0.0),
                    locked: false,
                    total: dec!(2.0),
                }
            )
        );

        assert_eq!(ledger.tx_log.len(), 4);

        Ok(())
    }

    #[test]
    fn process_tx_dispute() -> Result<()> {
        let mut ledger = Ledger::new();

        let _ = ledger.process_tx(Transaction::Deposit {
            amount: dec!(10.0),
            client: 1,
            tx: 1,
        });

        let _ = ledger.process_tx(Transaction::Dispute { client: 1, tx: 1 });

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(10.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 2);

        Ok(())
    }

    #[test]
    fn process_tx_dispute_tx_not_found() -> Result<()> {
        let mut ledger = Ledger::new();

        let _ = ledger.process_tx(Transaction::Deposit {
            amount: dec!(10.0),
            client: 1,
            tx: 1,
        });

        let result = ledger.process_tx(Transaction::Dispute { client: 1, tx: 3 });

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(LedgerError::TransactionNotFound { tx: 3 })
        ));

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.available, dec!(10.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }
}
