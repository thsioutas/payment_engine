use crate::accounts::ClientInfoStorage;
use crate::transactions::{Transaction, TransactionError};

/// The main struct of the payment engine. Contains the complete client storage
pub struct PaymentEngine {
    client_storage: ClientInfoStorage,
}

impl PaymentEngine {
    /// Runs the Payment Engine
    pub fn run(transactions: impl Iterator<Item = Result<Transaction, TransactionError>>) -> Self {
        let mut client_storage = ClientInfoStorage::new();
        for transaction_result in transactions {
            match transaction_result {
                Ok(transaction) => {
                    log::debug!("{:?}", transaction);
                    client_storage.update(transaction);
                }
                Err(error) => {
                    log::error!("Failed to deserialize transaction: {:?}", error);
                }
            }
        }
        Self { client_storage }
    }

    /// Outputs the stored accounts to a CSV format to stdout
    pub fn output_to_csv_format(&self, writer: impl std::io::Write) {
        let records = self.client_storage.get_csv_format_accounts();
        let mut csv_writer = csv::Writer::from_writer(writer);
        for record in records {
            let _ = csv_writer.serialize(record);
        }
    }
}
