use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::ClientId;
use crate::account::Account;
use crate::tx::{DomesticTransaction, SupportTransaction, Transaction};

pub type Result<T> = std::result::Result<T, LedgerError>;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Account not found for client: {tx:?}")]
    AccountNotFound { tx: Transaction },
    #[error("Insufficient funds to perform transaction: {tx:?}")]
    InsufficientFunds { tx: Transaction },
    #[error("Transaction not found: {tx:?}")]
    TransactionNotFound { tx: Transaction },
    #[error("Invalid Transaction for Dispute: {tx:?}")]
    InvalidTransactionForDispute { tx: Transaction },
    #[error("Dispute Transaction not found: {tx:?}. No dispute in progress.")]
    DisputeTxNotFound { tx: Transaction },
    #[error("Account {client_id}, is locked and cannot process transaction: {tx:?}")]
    LockedAccount {
        client_id: ClientId,
        tx: Transaction,
    },
    #[error("Account {client_id}, has inconsistent held funds: {tx:?}")]
    IncosistentHeldFunds {
        client_id: ClientId,
        tx: Transaction,
    },
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
            Transaction::Deposit(domestic_tx) => self.handle_deposit(domestic_tx),
            Transaction::Withdrawal(domestic_tx) => self.handle_withdrawal(domestic_tx),
            Transaction::Dispute(support_tx) => self.handle_dispute(support_tx),
            Transaction::Resolve(support_tx) => self.handle_resolve(support_tx),
            Transaction::Chargeback(support_tx) => self.handle_chargeback(support_tx),
        }
    }

    #[inline(always)]
    fn handle_deposit(&mut self, domestic_tx: DomesticTransaction) -> Result<()> {
        let account = self.accounts.entry(domestic_tx.client).or_default();

        if account.locked {
            return Err(LedgerError::LockedAccount {
                client_id: domestic_tx.client,
                tx: Transaction::Deposit(domestic_tx),
            });
        }

        account.available += domestic_tx.amount;
        account.total += domestic_tx.amount;

        self.tx_log.insert(Transaction::Deposit(domestic_tx));

        Ok(())
    }

    #[inline(always)]
    fn handle_withdrawal(&mut self, domestic_tx: DomesticTransaction) -> Result<()> {
        let Some(account) = self.accounts.get_mut(&domestic_tx.client) else {
            return Err(LedgerError::AccountNotFound {
                tx: Transaction::Withdrawal(domestic_tx),
            });
        };

        if account.locked {
            return Err(LedgerError::LockedAccount {
                client_id: domestic_tx.client,
                tx: Transaction::Withdrawal(domestic_tx),
            });
        }

        if account.available >= domestic_tx.amount {
            account.available -= domestic_tx.amount;
            account.total -= domestic_tx.amount;

            self.tx_log.insert(Transaction::Withdrawal(domestic_tx));

            return Ok(());
        }

        Err(LedgerError::InsufficientFunds {
            tx: Transaction::Withdrawal(domestic_tx),
        })
    }

    #[inline(always)]
    fn handle_dispute(&mut self, support_tx: SupportTransaction) -> Result<()> {
        let Some(tx_under_dispute) = self
            .find_tx(|t| t.id() == support_tx.tx && t.client_id() == support_tx.client)
            .cloned()
        else {
            return Err(LedgerError::TransactionNotFound {
                tx: Transaction::Dispute(support_tx),
            });
        };

        let Some(account) = self.accounts.get_mut(&support_tx.client) else {
            return Err(LedgerError::AccountNotFound {
                tx: Transaction::Dispute(support_tx),
            });
        };

        if account.locked {
            return Err(LedgerError::LockedAccount {
                client_id: support_tx.client,
                tx: Transaction::Dispute(support_tx),
            });
        }

        let amount_disputed = match tx_under_dispute {
            Transaction::Deposit(DomesticTransaction { amount, .. }) => amount,
            Transaction::Withdrawal(DomesticTransaction { amount, .. }) => amount,
            _ => {
                return Err(LedgerError::InvalidTransactionForDispute {
                    tx: Transaction::Dispute(support_tx),
                });
            }
        };

        if account.available >= amount_disputed {
            account.available -= amount_disputed;
            account.held += amount_disputed;
        } else {
            return Err(LedgerError::InsufficientFunds {
                tx: Transaction::Dispute(support_tx),
            });
        }

        self.tx_log.insert(Transaction::Dispute(support_tx));

        Ok(())
    }

    #[inline(always)]
    fn handle_resolve(&mut self, support_tx: SupportTransaction) -> Result<()> {
        let Some(tx_under_dispute) = self
            .find_tx(|t| t.id() == support_tx.tx && t.client_id() == support_tx.client)
            .cloned()
        else {
            return Err(LedgerError::TransactionNotFound {
                tx: Transaction::Resolve(support_tx),
            });
        };

        let Some(account) = self.accounts.get_mut(&support_tx.client) else {
            return Err(LedgerError::AccountNotFound {
                tx: Transaction::Resolve(support_tx),
            });
        };

        if account.locked {
            return Err(LedgerError::LockedAccount {
                client_id: support_tx.client,
                tx: Transaction::Resolve(support_tx),
            });
        }

        let amount_resolved = match tx_under_dispute {
            Transaction::Deposit(DomesticTransaction { amount, .. }) => amount,
            Transaction::Withdrawal(DomesticTransaction { amount, .. }) => amount,
            _ => {
                return Err(LedgerError::InvalidTransactionForDispute {
                    tx: Transaction::Resolve(support_tx),
                });
            }
        };

        if account.held >= amount_resolved {
            account.held -= amount_resolved;
            account.available += amount_resolved;
        } else {
            return Err(LedgerError::IncosistentHeldFunds {
                client_id: support_tx.client,
                tx: Transaction::Resolve(support_tx),
            });
        }

        self.tx_log.insert(Transaction::Resolve(support_tx));

        Ok(())
    }

    #[inline(always)]
    fn handle_chargeback(&mut self, support_tx: SupportTransaction) -> Result<()> {
        let Some(tx_under_dispute) = self
            .find_tx(|t| t.id() == support_tx.tx && t.client_id() == support_tx.client)
            .cloned()
        else {
            return Err(LedgerError::TransactionNotFound {
                tx: Transaction::Chargeback(support_tx),
            });
        };

        let Some(account) = self.accounts.get_mut(&support_tx.client) else {
            return Err(LedgerError::AccountNotFound {
                tx: Transaction::Chargeback(support_tx),
            });
        };

        if account.locked {
            return Err(LedgerError::LockedAccount {
                client_id: support_tx.client,
                tx: Transaction::Chargeback(support_tx),
            });
        }

        let amount_chargeback = match tx_under_dispute {
            Transaction::Deposit(DomesticTransaction { amount, .. }) => amount,
            Transaction::Withdrawal(DomesticTransaction { amount, .. }) => amount,
            _ => {
                return Err(LedgerError::InvalidTransactionForDispute {
                    tx: Transaction::Chargeback(support_tx),
                });
            }
        };

        if account.held >= amount_chargeback {
            account.held -= amount_chargeback;
            account.total -= amount_chargeback;
            account.locked = true;
        } else {
            return Err(LedgerError::IncosistentHeldFunds {
                client_id: support_tx.client,
                tx: Transaction::Chargeback(support_tx),
            });
        }

        self.tx_log.insert(Transaction::Chargeback(support_tx));

        Ok(())
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

        ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(100.0),
            client: 1,
            tx: 1,
        }))?;

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(100.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_deposit_withdrawal() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(100.0),
            client: 1,
            tx: 1,
        }))?;

        ledger.process_tx(Transaction::Withdrawal(DomesticTransaction {
            amount: dec!(100.0),
            client: 1,
            tx: 2,
        }))?;

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(0.0));
        assert_eq!(ledger.tx_log.len(), 2);

        Ok(())
    }

    #[test]
    fn process_tx_withdrawal_handles_insufficient_funds() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(2.0),
            client: 2,
            tx: 1,
        }))?;

        let tx = Transaction::Withdrawal(DomesticTransaction {
            amount: dec!(3.0),
            client: 2,
            tx: 2,
        });
        let result = ledger.process_tx(tx.clone());

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(LedgerError::InsufficientFunds { tx: _ })
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

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(1.0),
            client: 1,
            tx: 1,
        }));

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(2.0),
            client: 2,
            tx: 2,
        }));

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(2.0),
            client: 1,
            tx: 3,
        }));

        let _ = ledger.process_tx(Transaction::Withdrawal(DomesticTransaction {
            amount: dec!(1.5),
            client: 1,
            tx: 4,
        }));

        let _ = ledger.process_tx(Transaction::Withdrawal(DomesticTransaction {
            amount: dec!(3.0),
            client: 2,
            tx: 5,
        }));

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

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(10.0),
            client: 1,
            tx: 1,
        }));

        let _ = ledger.process_tx(Transaction::Dispute(SupportTransaction {
            client: 1,
            tx: 1,
        }));

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

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(10.0),
            client: 1,
            tx: 1,
        }));

        let tx = Transaction::Dispute(SupportTransaction { client: 1, tx: 3 });
        let result = ledger.process_tx(tx.clone());

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(LedgerError::TransactionNotFound { tx: _ })
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

    #[test]
    fn process_tx_dispute_resolve() -> Result<()> {
        let mut ledger = Ledger::new();

        let _ = ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(10.0),
            client: 1,
            tx: 1,
        }));

        let _ = ledger.process_tx(Transaction::Dispute(SupportTransaction {
            client: 1,
            tx: 1,
        }));

        let _ = ledger.process_tx(Transaction::Resolve(SupportTransaction {
            client: 1,
            tx: 1,
        }));

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert_eq!(account.available, dec!(10.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 3);

        Ok(())
    }

    #[test]
    fn process_tx_dispute_chargeback() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction::Deposit(DomesticTransaction {
            amount: dec!(100.0),
            client: 1,
            tx: 1,
        }))?;

        ledger.process_tx(Transaction::Withdrawal(DomesticTransaction {
            amount: dec!(100.0),
            client: 1,
            tx: 2,
        }))?;

        ledger.process_tx(Transaction::Dispute(SupportTransaction {
            client: 1,
            tx: 2,
        }))?;

        ledger.process_tx(Transaction::Chargeback(SupportTransaction {
            client: 1,
            tx: 2,
        }))?;

        let account = ledger
            .get_account(&1)
            .expect("expected account for client.");

        assert!(account.locked);
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(0.0));
        assert_eq!(ledger.tx_log.len(), 4);

        Ok(())
    }
}
