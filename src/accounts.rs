use serde::Serialize;

use crate::transactions::{Amount, ClientId, Transaction, TransactionId};
use std::collections::HashMap;

type TransactionLogId = (ClientId, TransactionId);

pub struct Account {
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

#[derive(Serialize)]
struct AccountRecord {
    client: ClientId,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool
}

impl Default for Account {
    fn default() -> Self {
        Self {
            available: 0.0,
            held: 0.0,
            locked: false,
        }
    }
}

impl Account {
    fn deposit(&mut self, amount: Amount) {
        self.available += amount;
    }

    fn withdraw(&mut self, amount: Amount) {
        self.available -= amount;
    }

    fn dispute(&mut self, amount: Amount) {
        self.available -= amount;
        self.held += amount;
    }

    fn resolve(&mut self, amount: Amount) {
        self.available += amount;
        self.held -= amount;
    }

    fn charge_back(&mut self, amount: Amount) {
        self.held -= amount;
        self.locked = true;
    }
}

pub struct Accounts {
    pub accounts: HashMap<ClientId, Account>,
    deposits: HashMap<TransactionLogId, Amount>,
    disputes: HashMap<TransactionLogId, Amount>,
}

impl Accounts {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            deposits: HashMap::new(),
            disputes: HashMap::new(),
        }
    }

    pub fn update(&mut self, transaction: Transaction) {
        use Transaction::*;
        match transaction {
            Deposit(info) => {
                self.accounts
                    .entry(info.client)
                    .or_insert_with(Account::default)
                    .deposit(info.amount);
                self.deposits.insert((info.client, info.tx), info.amount);
            }
            Withdrawal(info) => {
                if let Some(account) = self.accounts.get_mut(&info.client) {
                    account.withdraw(info.amount);
                } else {
                    log::error!("Withdraw transaction for unavailable client ID");
                }
            }
            Dispute(info) => {
                let transaction_log_id = (info.client, info.tx);
                if let Some(amount) = self.deposits.get(&transaction_log_id) {
                    if let Some(account) = self.accounts.get_mut(&info.client) {
                        account.dispute(*amount);
                        self.disputes.insert(transaction_log_id, *amount);
                    } else {
                        log::error!("Not available account for dispute transaction")
                    }
                } else {
                    log::error!("Not available deposit transaction to be disputed")
                }
            }
            Resolve(info) => {
                let transaction_log_id = (info.client, info.tx);
                if let Some(amount) = self.disputes.remove(&transaction_log_id) {
                    if let Some(account) = self.accounts.get_mut(&info.client) {
                        account.resolve(amount);
                    } else {
                        log::error!("Not available account for resolved transaction");
                    }
                } else {
                    log::error!("Not available disputed transaction to be resolved");
                }
            }
            ChargeBack(info) => {
                let transaction_log_id = (info.client, info.tx);
                if let Some(amount) = self.disputes.remove(&transaction_log_id) {
                    if let Some(account) = self.accounts.get_mut(&info.client) {
                        account.charge_back(amount);
                    } else {
                        log::error!("Not available account for resolved transaction");
                    }
                } else {
                    log::error!("Not available disputed transaction to be resolved");
                }
            }
        }
    }

    pub fn output(&self) {
        let records = self.accounts.iter().map(|(client, account)| AccountRecord { 
            client: *client, 
            available: account.available,
            held: account.held, 
            total: account.available + account.held, 
            locked: account.locked, 
        });
        let mut csv_writer = csv::Writer::from_writer(std::io::stdout());
        for record in records {
            let _ = csv_writer.serialize(record);
        }
    }
}
