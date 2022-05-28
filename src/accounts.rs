use serde::Serialize;

use crate::transactions::{Amount, ClientId, Transaction, TransactionId};
use std::collections::HashMap;

/// Holds all the necessary info of an account for the output CSV
#[derive(Serialize, Debug, PartialEq)]
pub struct CsvAccount {
    client: ClientId,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
}

/// Helper struct which holds the necessary info of an account for the ClientInfoStorage
#[derive(Clone, Copy)]
struct Account {
    available: Amount,
    held: Amount,
    locked: bool,
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

fn round_to_4_dec(amount: Amount) -> Amount {
    (amount * 10000.0).round() / 10000.0
}

impl Account {
    fn deposit(&mut self, amount: Amount) -> Self {
        if amount >= 0.0 {
            self.available += amount;
        } else {
            log::error!("Do not process negative amounts");
        }
        *self
    }

    fn withdraw(&mut self, amount: Amount) {
        if amount >= 0.0 {
            let possible_available = self.available - amount;
            if possible_available >= 0.0 {
                self.available = possible_available;
            } else {
                log::error!("Not enough funds");
            }
        } else {
            log::error!("Do not process negative amounts");
        }
    }

    fn dispute(&mut self, amount: Amount) {
        // Amount should always be >= 0 here
        self.available -= amount;
        self.held += amount;
    }

    fn resolve(&mut self, amount: Amount) {
        // Amount should always be >= 0 here
        self.available += amount;
        self.held -= amount;
    }

    fn charge_back(&mut self, amount: Amount) {
        // Amount should always be >= 0 here
        self.held -= amount;
        self.locked = true;
    }
}

struct DepositLog {
    amount: Amount,
    disputed: bool,
}

/// Stores the current state of available clients, their accounts and their deposits
pub struct ClientInfoStorage {
    client_info: HashMap<ClientId, (Account, HashMap<TransactionId, DepositLog>)>,
}

// clippy suggestion
impl Default for ClientInfoStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientInfoStorage {
    /// Creates a new ClientInfoStorage
    pub fn new() -> Self {
        Self {
            client_info: HashMap::new(),
        }
    }

    /// Updates the AccountStorage based on the input Transaction
    pub fn update(&mut self, transaction: Transaction) {
        use Transaction::*;
        match transaction {
            Deposit(info) => {
                if let Some(client_info) = self.client_info.get_mut(&info.client) {
                    if client_info.0.locked {
                        log::warn!("Client's account  ({}) is locked", info.client);
                    } else {
                        client_info.1.insert(
                            info.tx,
                            DepositLog {
                                amount: info.amount,
                                disputed: false,
                            },
                        );
                        client_info.0.deposit(info.amount);
                    }
                } else {
                    let mut new_entry = HashMap::new();
                    new_entry.insert(
                        info.tx,
                        DepositLog {
                            amount: info.amount,
                            disputed: false,
                        },
                    );
                    self.client_info.insert(
                        info.client,
                        (Account::default().deposit(info.amount), new_entry),
                    );
                }
            }
            Withdrawal(info) => {
                if let Some(client_info) = self.client_info.get_mut(&info.client) {
                    if client_info.0.locked {
                        log::warn!("Client's account  ({}) is locked", info.client);
                    } else {
                        client_info.0.withdraw(info.amount);
                    }
                } else {
                    log::error!("Withdraw transaction for unavailable client ID");
                }
            }
            Dispute(info) => {
                if let Some(client_info) = self.client_info.get_mut(&info.client) {
                    if client_info.0.locked {
                        log::warn!("Client's account  ({}) is locked", info.client);
                    } else if let Some(deposit) = client_info.1.get_mut(&info.tx) {
                        if !deposit.disputed {
                            let amount = deposit.amount;
                            deposit.disputed = true;
                            client_info.0.dispute(amount);
                        } else {
                            log::error!("Dispute error: deposit already disputed")
                        }
                    } else {
                        log::error!("Not available deposit to be disputed");
                    }
                } else {
                    log::error!("Not available client for resolved transaction");
                }
            }
            Resolve(info) => {
                if let Some(client_info) = self.client_info.get_mut(&info.client) {
                    if client_info.0.locked {
                        log::warn!("Client's account  ({}) is locked", info.client);
                    } else if let Some(deposit) = client_info.1.get_mut(&info.tx) {
                        if !deposit.disputed {
                            log::error!("Resolve error: Deposit has not been disputed");
                        } else {
                            let amount = deposit.amount;
                            deposit.disputed = false;
                            client_info.0.resolve(amount);
                        }
                    } else {
                        log::error!("Resolve error: Not available disputed deposit to be resolved");
                    }
                } else {
                    log::error!("Resolve error: Not available client for resolved transaction");
                }
            }
            ChargeBack(info) => {
                if let Some(client_info) = self.client_info.get_mut(&info.client) {
                    if client_info.0.locked {
                        log::warn!("Client's account  ({}) is locked", info.client);
                    } else if let Some(deposit) = client_info.1.get_mut(&info.tx) {
                        if !deposit.disputed {
                            log::error!("ChargeBack error: Deposit has not been disputed");
                        } else {
                            let amount = deposit.amount;
                            deposit.disputed = false;
                            client_info.0.charge_back(amount);
                        }
                    } else {
                        log::error!(
                            "ChargeBack error: Not available disputed deposit to be resolved"
                        );
                    }
                } else {
                    log::error!(
                        "ChargeBack error: Not available client for charge-back transaction"
                    );
                }
            }
        }
    }

    /// Outputs the stored accounts to a CSV format
    pub fn get_csv_format_accounts(&self) -> Vec<CsvAccount> {
        let records = self
            .client_info
            .iter()
            .map(|(client, client_info)| CsvAccount {
                client: *client,
                available: round_to_4_dec(client_info.0.available),
                held: round_to_4_dec(client_info.0.held),
                total: round_to_4_dec(client_info.0.available + client_info.0.held),
                locked: client_info.0.locked,
            })
            .collect();
        records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::{
        ChargeBackInfo, DepositInfo, DisputeInfo, ResolveInfo, Transaction, WithdrawalInfo,
    };
    #[test]
    fn test_client_info() {
        let mut client_storage = ClientInfoStorage::new();

        // Test 1st deposit
        let transaction = Transaction::Deposit(DepositInfo {
            client: 2,
            tx: 1,
            amount: 1.2345,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 1.2345,
            held: 0.0,
            total: 1.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test 2nd deposit
        let transaction = Transaction::Deposit(DepositInfo {
            client: 2,
            tx: 2,
            amount: 2.0001,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 3.2346,
            held: 0.0,
            total: 3.2346,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test Withdraw
        let transaction = Transaction::Withdrawal(WithdrawalInfo {
            client: 2,
            tx: 3,
            amount: 1.0001,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 2.2345,
            held: 0.0,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test Dispute (tx = 2)
        let transaction = Transaction::Dispute(DisputeInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 0.2344,
            held: 2.0001,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Dispute second time the same transaction (tx = 2)
        let transaction = Transaction::Dispute(DisputeInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 0.2344,
            held: 2.0001,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Resolve (tx = 2)
        let transaction = Transaction::Resolve(ResolveInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 2.2345,
            held: 0.0,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Resolve un-disputed (tx = 2)
        let transaction = Transaction::Resolve(ResolveInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 2.2345,
            held: 0.0,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Charge back un-disputed (tx = 2)
        let transaction = Transaction::ChargeBack(ChargeBackInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 2.2345,
            held: 0.0,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test Dispute (tx = 1)
        let transaction = Transaction::Dispute(DisputeInfo { client: 2, tx: 1 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 2,
            available: 1.0,
            held: 1.2345,
            total: 2.2345,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Charge back disputed (tx = 1)
        let transaction = Transaction::ChargeBack(ChargeBackInfo { client: 2, tx: 1 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_locked_records = CsvAccount {
            client: 2,
            available: 1.0,
            held: 0.0,
            total: 1.0,
            locked: true,
        };
        assert_eq!(records[0], expected_locked_records);

        // Test deposit on locked account
        let transaction = Transaction::Deposit(DepositInfo {
            client: 2,
            tx: 1,
            amount: 1.2345,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records[0], expected_locked_records);

        // Test withdraw on locked account
        let transaction = Transaction::Withdrawal(WithdrawalInfo {
            client: 2,
            tx: 3,
            amount: 1.0001,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records[0], expected_locked_records);

        // Test Dispute on locked account (tx = 2)
        let transaction = Transaction::Dispute(DisputeInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records[0], expected_locked_records);

        // Resolve (tx = 2) on locked account
        let transaction = Transaction::Resolve(ResolveInfo { client: 2, tx: 2 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records[0], expected_locked_records);

        // Charge back (tx = 1) on locked account
        let transaction = Transaction::ChargeBack(ChargeBackInfo { client: 2, tx: 1 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records[0], expected_locked_records);
    }

    #[test]
    fn test_negative_failed_transactions() {
        let mut client_storage = ClientInfoStorage::new();
        let transaction = Transaction::Deposit(DepositInfo {
            client: 1,
            tx: 1,
            amount: 12345.12,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 1,
            available: 12345.12,
            held: 0.0,
            total: 12345.12,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test deposit of negative amount
        let transaction = Transaction::Deposit(DepositInfo {
            client: 1,
            tx: 1,
            amount: -12345.12,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 1,
            available: 12345.12,
            held: 0.0,
            total: 12345.12,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test Withdraw of negative amount
        let transaction = Transaction::Withdrawal(WithdrawalInfo {
            client: 1,
            tx: 3,
            amount: -1.0001,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 1,
            available: 12345.12,
            held: 0.0,
            total: 12345.12,
            locked: false,
        };
        assert_eq!(records[0], expected_records);

        // Test Withdraw of insufficient funds
        let transaction = Transaction::Withdrawal(WithdrawalInfo {
            client: 1,
            tx: 3,
            amount: 5199999.123,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        let expected_records = CsvAccount {
            client: 1,
            available: 12345.12,
            held: 0.0,
            total: 12345.12,
            locked: false,
        };
        assert_eq!(records[0], expected_records);
    }

    #[test]
    fn test_client_info_not_registered_client() {
        let mut client_storage = ClientInfoStorage::new();
        // Test Withdraw
        let transaction = Transaction::Withdrawal(WithdrawalInfo {
            client: 2,
            tx: 3,
            amount: 1.0001,
        });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records.is_empty(), true);

        let mut client_storage = ClientInfoStorage::new();
        // Test Dispute
        let transaction = Transaction::Dispute(DisputeInfo { client: 2, tx: 3 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records.is_empty(), true);

        // Test Resolve
        let transaction = Transaction::Resolve(ResolveInfo { client: 2, tx: 3 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records.is_empty(), true);

        // Test Chardge back
        let transaction = Transaction::ChargeBack(ChargeBackInfo { client: 2, tx: 3 });
        client_storage.update(transaction);
        let records = client_storage.get_csv_format_accounts();
        assert_eq!(records.is_empty(), true);
    }

    #[test]
    fn test_client_info_flow() {
        let mut client_storage = ClientInfoStorage::new();
        let transactions = vec![
            Transaction::Deposit(DepositInfo {
                client: 2,
                tx: 1,
                amount: 1.0,
            }),
            Transaction::Deposit(DepositInfo {
                client: 1,
                tx: 2,
                amount: 2.0,
            }),
            Transaction::Withdrawal(WithdrawalInfo {
                client: 2,
                tx: 3,
                amount: 0.5,
            }),
            Transaction::Withdrawal(WithdrawalInfo {
                client: 1,
                tx: 4,
                amount: 1.2,
            }),
            Transaction::Withdrawal(WithdrawalInfo {
                client: 2,
                tx: 5,
                amount: 3.0,
            }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.5) - held (0.0) - total (0.5) - locked (false)
            Transaction::Dispute(DisputeInfo { client: 2, tx: 1 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (-0.5) - held (1.0) - total (0.5) - locked (false)
            Transaction::Dispute(DisputeInfo { client: 2, tx: 1 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (-0.5) - held (1.0) - total (0.5) - locked (false)
            Transaction::Dispute(DisputeInfo { client: 3, tx: 1 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (-0.5) - held (1.0) - total (0.5) - locked (false)
            Transaction::Dispute(DisputeInfo { client: 2, tx: 5 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (-0.5) - held (1.0) - total (0.5) - locked (false)
            Transaction::Resolve(ResolveInfo { client: 2, tx: 1 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.5) - held (0) - total (0.5) - locked (false)
            Transaction::Deposit(DepositInfo {
                client: 2,
                tx: 6,
                amount: 0.1,
            }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.6) - held (0) - total (0.6) - locked (false)
            Transaction::Dispute(DisputeInfo { client: 2, tx: 6 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.5) - held (0.1) - total (0.6) - locked (false)
            Transaction::ChargeBack(ChargeBackInfo { client: 2, tx: 6 }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.5) - held (0.0) - total (0.5) - locked (true)
            Transaction::Deposit(DepositInfo {
                client: 2,
                tx: 7,
                amount: 1.0,
            }),
            // Client 1: available (0.8) - held (0.0) - total (0.8) - locked (false)
            // Client 2: available (0.5) - held (0.0) - total (0.5) - locked (true)
        ];
        for transaction in transactions {
            client_storage.update(transaction);
        }
        let records = client_storage.get_csv_format_accounts();
        let expected_records_1 = CsvAccount {
            client: 1,
            available: 0.8,
            held: 0.0,
            total: 0.8,
            locked: false,
        };
        let expected_records_2 = CsvAccount {
            client: 2,
            available: 0.5,
            held: 0.0,
            total: 0.5,
            locked: true,
        };
        if records[1].client == 1 {
            assert_eq!(records[1], expected_records_1);
            assert_eq!(records[0], expected_records_2);
        } else {
            assert_eq!(records[0], expected_records_1);
            assert_eq!(records[1], expected_records_2);
        }
    }
}
