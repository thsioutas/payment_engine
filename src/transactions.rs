use serde::Deserialize;

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Amount = f32;

#[derive(Deserialize, Debug)]
struct CsvTransaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Option<Amount>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    ChargeBack,
}

#[derive(Debug)]
pub enum Transaction {
    Deposit(DepositInfo),
    Withdrawal(WithdrawalInfo),
    Dispute(DisputeInfo),
    Resolve(ResolveInfo),
    ChargeBack(ChargeBackInfo),
}

#[derive(Debug)]
pub struct DepositInfo {
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Amount,
}

#[derive(Debug)]
pub struct WithdrawalInfo {
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Amount,
}

#[derive(Debug)]
pub struct DisputeInfo {
    pub client: ClientId,
    pub tx: TransactionId,
}

#[derive(Debug)]
pub struct ResolveInfo {
    pub client: ClientId,
    pub tx: TransactionId,
}

#[derive(Debug)]
pub struct ChargeBackInfo {
    pub client: ClientId,
    pub tx: TransactionId,
}

impl TryFrom<CsvTransaction> for Transaction {
    type Error = TransactionError;
    fn try_from(csv_transaction: CsvTransaction) -> Result<Self, Self::Error> {
        use TransactionType::*;
        match csv_transaction.transaction_type {
            Deposit => {
                if let Some(amount) = csv_transaction.amount {
                    Ok(Transaction::Deposit(DepositInfo {
                        client: csv_transaction.client,
                        tx: csv_transaction.tx,
                        amount,
                    }))
                } else {
                    Err(TransactionError::WrongFormat)
                }
            }
            Withdrawal => {
                if let Some(amount) = csv_transaction.amount {
                    Ok(Transaction::Withdrawal(WithdrawalInfo {
                        client: csv_transaction.client,
                        tx: csv_transaction.tx,
                        amount,
                    }))
                } else {
                    Err(TransactionError::WrongFormat)
                }
            }
            Dispute => {
                // Intentionally ignore amount if present. Do not consider it an error
                Ok(Transaction::Dispute(DisputeInfo {
                    client: csv_transaction.client,
                    tx: csv_transaction.tx,
                }))
            }
            Resolve => {
                // Intentionally ignore amount if present. Do not consider it an error
                Ok(Transaction::Resolve(ResolveInfo {
                    client: csv_transaction.client,
                    tx: csv_transaction.tx,
                }))
            }
            ChargeBack => {
                // Intentionally ignore amount if present. Do not consider it an error
                Ok(Transaction::ChargeBack(ChargeBackInfo {
                    client: csv_transaction.client,
                    tx: csv_transaction.tx,
                }))
            }
        }
    }
}

/// Read transactions from input reader
pub fn read_transactions(
    reader: impl std::io::Read,
) -> impl Iterator<Item = Result<Transaction, TransactionError>> {
    let reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    let csv_transactions = reader.into_deserialize::<CsvTransaction>();
    csv_transactions.map(|csv_transaction_result| {
        csv_transaction_result
            .map_err(|_| TransactionError::CsvDeserializeError)
            .and_then(|csv_transaction| csv_transaction.try_into())
    })
}

#[derive(Debug)]
pub enum TransactionError {
    CsvDeserializeError,
    WrongFormat,
}
