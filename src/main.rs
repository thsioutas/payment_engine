use log::info;
use payment_engine::engine::PaymentEngine;
use payment_engine::transactions::read_transactions;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    /// Transaction input file path.
    #[structopt(parse(from_os_str))]
    input_file_path: PathBuf,
}

/// Entrypoint of the application
fn main() {
    let log_file = File::create("log.txt").expect("Unable to open log file");
    let _ = simplelog::WriteLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        Box::new(log_file),
    );
    info!("Start toy payment engine!");
    let args = Opt::from_args();

    let input_file = File::open(args.input_file_path).expect("Unable to open input file");

    // Read transactions from CSV
    let transactions = read_transactions(input_file);
    // Run payment engine for the given transactions
    let payment_engine = PaymentEngine::run(transactions);
    // Output the payment engine's results in a CSV format to stdout
    payment_engine.output_to_csv_format(std::io::stdout());
}
