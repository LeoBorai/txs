use std::collections::{HashMap, HashSet};

use crate::ClientId;
use crate::account::Account;
use crate::error::{Error, Result};
use crate::tx::{Transaction, TransactionType};

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

    pub fn find_tx<P>(&self, p: P) -> Option<&Transaction>
    where
        P: Fn(&&Transaction) -> bool,
    {
        self.tx_log.iter().find(p)
    }

    pub fn accounts_iter(&self) -> impl Iterator<Item = (&ClientId, &Account)> {
        self.accounts.iter()
    }

    pub fn accounts_summary(&self) -> Vec<Account> {
        let accounts: Vec<Account> = self
            .accounts_iter()
            .map(|(id, acct)| Account {
                id: *id,
                available: acct.available,
                held: acct.held,
                locked: acct.locked,
                total: acct.total,
            })
            .collect();

        accounts
    }

    pub fn process_tx(&mut self, tx: Transaction) -> Result<()> {
        match tx.r#type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        }
    }

    #[inline(always)]
    fn handle_deposit(&mut self, tx: Transaction) -> Result<()> {
        let account = self.accounts.entry(tx.client).or_default();

        if account.locked {
            return Err(Error::LockedAccount { tx });
        }

        let amount = tx.amount()?;

        account.available += amount;
        account.total += amount;

        self.tx_log.insert(tx);

        Ok(())
    }

    #[inline(always)]
    fn handle_withdrawal(&mut self, tx: Transaction) -> Result<()> {
        let Some(account) = self.accounts.get_mut(&tx.client) else {
            return Err(Error::AccountNotFound { tx });
        };

        if account.locked {
            return Err(Error::LockedAccount { tx });
        }

        let amount = tx.amount()?;

        if account.available >= amount {
            account.available -= amount;
            account.total -= amount;

            self.tx_log.insert(tx);

            return Ok(());
        }

        Err(Error::InsufficientFunds { tx })
    }

    #[inline(always)]
    fn handle_dispute(&mut self, tx: Transaction) -> Result<()> {
        let Some(tx_under_dispute) = self
            .find_tx(|t| t.id == tx.id && t.client == tx.client)
            .cloned()
        else {
            return Err(Error::TransactionNotFound { tx });
        };

        let Some(account) = self.accounts.get_mut(&tx.client) else {
            return Err(Error::AccountNotFound { tx });
        };

        if account.locked {
            return Err(Error::LockedAccount { tx });
        }

        let amount_disputed = tx_under_dispute.amount()?;

        if account.available >= amount_disputed {
            account.available -= amount_disputed;
            account.held += amount_disputed;
        } else {
            return Err(Error::InsufficientFunds { tx });
        }

        self.tx_log.insert(tx);

        Ok(())
    }

    #[inline(always)]
    fn handle_resolve(&mut self, tx: Transaction) -> Result<()> {
        if self
            .find_tx(|t| {
                t.id == tx.id
                    && t.client == tx.client
                    && matches!(t.r#type, TransactionType::Dispute)
            })
            .is_none()
        {
            return Err(Error::DisputeTxNotFound { tx });
        }

        let Some(tx_under_dispute) = self
            .find_tx(|t| {
                t.id == tx.id
                    && t.client == tx.client
                    && matches!(
                        t.r#type,
                        TransactionType::Deposit | TransactionType::Withdrawal
                    )
            })
            .cloned()
        else {
            return Err(Error::TransactionNotFound { tx });
        };

        let Some(account) = self.accounts.get_mut(&tx.client) else {
            return Err(Error::AccountNotFound { tx });
        };

        if account.locked {
            return Err(Error::LockedAccount { tx });
        }

        let amount_resolved = tx_under_dispute.amount()?;

        if account.held >= amount_resolved {
            account.held -= amount_resolved;
            account.available += amount_resolved;
        } else {
            return Err(Error::IncosistentHeldFunds { tx });
        }

        self.tx_log.insert(tx);

        Ok(())
    }

    #[inline(always)]
    fn handle_chargeback(&mut self, tx: Transaction) -> Result<()> {
        if self
            .find_tx(|t| {
                t.id == tx.id
                    && t.client == tx.client
                    && matches!(t.r#type, TransactionType::Dispute)
            })
            .is_none()
        {
            return Err(Error::DisputeTxNotFound { tx });
        };

        let Some(tx_under_dispute) = self
            .find_tx(|t| {
                t.id == tx.id
                    && t.client == tx.client
                    && matches!(
                        t.r#type,
                        TransactionType::Deposit | TransactionType::Withdrawal
                    )
            })
            .cloned()
        else {
            return Err(Error::TransactionNotFound { tx });
        };

        let Some(account) = self.accounts.get_mut(&tx.client) else {
            return Err(Error::AccountNotFound { tx });
        };

        if account.locked {
            return Err(Error::LockedAccount { tx });
        }

        let amount_chargeback = tx_under_dispute.amount()?;

        if account.held >= amount_chargeback {
            account.held -= amount_chargeback;
            account.total -= amount_chargeback;
            account.locked = true;
        } else {
            return Err(Error::IncosistentHeldFunds { tx });
        }

        self.tx_log.insert(tx);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    pub fn get_account(ledger: &mut Ledger, client_id: ClientId) -> Option<&Account> {
        ledger.accounts.get(&client_id)
    }

    #[test]
    fn ledger_is_empty_by_default() {
        let ledger = Ledger::new();
        assert!(ledger.accounts.is_empty());
    }

    #[test]
    fn process_tx_deposit() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(100.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert_eq!(account.available, dec!(100.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_deposit_withdrawal() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(100.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: Some(dec!(100.0)),
            r#type: TransactionType::Withdrawal,
            client: 1,
            id: 2,
        })?;

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert_eq!(account.available, dec!(0.0));
        assert_eq!(ledger.tx_log.len(), 2);

        Ok(())
    }

    #[test]
    fn process_tx_withdrawal_handles_insufficient_funds() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(2.0)),
            r#type: TransactionType::Deposit,
            client: 2,
            id: 1,
        })?;

        let tx = Transaction {
            amount: Some(dec!(3.0)),
            r#type: TransactionType::Withdrawal,
            client: 2,
            id: 2,
        };
        let result = ledger.process_tx(tx.clone());

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::InsufficientFunds { tx: _ })));

        let account = get_account(&mut ledger, 2).expect("expected account for client.");

        assert_eq!(account.available, dec!(2.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_two_accounts() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(1.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: Some(dec!(2.0)),
            r#type: TransactionType::Deposit,
            client: 2,
            id: 2,
        })?;

        ledger.process_tx(Transaction {
            amount: Some(dec!(2.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 3,
        })?;

        ledger.process_tx(Transaction {
            amount: Some(dec!(1.5)),
            r#type: TransactionType::Withdrawal,
            client: 1,
            id: 4,
        })?;

        let result = ledger.process_tx(Transaction {
            amount: Some(dec!(3.0)),
            r#type: TransactionType::Withdrawal,
            client: 2,
            id: 5,
        });

        assert!(result.is_err(), "should fail due to insufficient funds");

        let mut accounts = ledger.accounts_iter().collect::<Vec<_>>();
        accounts.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(b_id));

        assert_eq!(
            accounts[0],
            (
                &1,
                &Account {
                    id: 1,
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
                    id: 2,
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

        ledger.process_tx(Transaction {
            amount: Some(dec!(10.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: None,
            r#type: TransactionType::Dispute,
            client: 1,
            id: 1,
        })?;

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert!(!account.locked);
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(10.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 2);

        Ok(())
    }

    #[test]
    fn process_tx_dispute_tx_not_found() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(10.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        let tx = Transaction {
            amount: None,
            r#type: TransactionType::Dispute,
            client: 1,
            id: 3,
        };
        let result = ledger.process_tx(tx.clone());

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::TransactionNotFound { tx: _ })));

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert!(!account.locked);
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.available, dec!(10.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 1);

        Ok(())
    }

    #[test]
    fn process_tx_dispute_resolve() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(10.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: None,
            r#type: TransactionType::Dispute,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: None,
            r#type: TransactionType::Resolve,
            client: 1,
            id: 1,
        })?;

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert!(!account.locked);
        assert_eq!(account.available, dec!(10.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(10.0));
        assert_eq!(ledger.tx_log.len(), 3);

        Ok(())
    }

    #[test]
    fn process_tx_dispute_deposit_chargeback() -> Result<()> {
        let mut ledger = Ledger::new();

        ledger.process_tx(Transaction {
            amount: Some(dec!(100.0)),
            r#type: TransactionType::Deposit,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: None,
            r#type: TransactionType::Dispute,
            client: 1,
            id: 1,
        })?;

        ledger.process_tx(Transaction {
            amount: None,
            r#type: TransactionType::Chargeback,
            client: 1,
            id: 1,
        })?;

        let account = get_account(&mut ledger, 1).expect("expected account for client.");

        assert!(account.locked);
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(0.0));
        assert_eq!(ledger.tx_log.len(), 3);

        Ok(())
    }
}
