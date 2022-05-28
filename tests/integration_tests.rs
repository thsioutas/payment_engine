use payment_engine::engine::PaymentEngine;
use payment_engine::transactions::read_transactions;

#[test]
fn integration_test() {
    let input_file =
        std::fs::File::open("example_inputs/transactions.csv").expect("Unable to open input file");
    let transactions = read_transactions(input_file);

    let engine = PaymentEngine::run(transactions);
    let output_file_path = "output.csv";
    engine.output_to_csv_format(std::fs::File::create(output_file_path).unwrap());

    let mut output = csv::Reader::from_reader(std::fs::File::open(output_file_path).unwrap());
    let records: Vec<csv::StringRecord> = output.records().flatten().collect();
    let client_1 = vec!["1", "0.8", "0.0", "0.8", "false"];
    let client_2 = vec!["2", "0.5", "0.0", "0.5", "true"];
    if records[0][0] == *"1" {
        assert_eq!(records[0], client_1);
        assert_eq!(records[1], client_2);
    } else {
        assert_eq!(records[1], client_1);
        assert_eq!(records[0], client_2);
    }
}
