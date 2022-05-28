# Payment engine

Implements a simple toy payments engine that reads a series of transactions
from a CSV, updates client accounts, handles disputes and chargebacks, and then outputs the
state of clients accounts as a CSV.

# Build instructions
The application builds against:
1.57.0-x86_64-unknown-linux-gnu (rustc 1.57)

# Run instructions
You should be able to run the payments engine like
```
cargo run -- transactions.csv > accounts.csv
```

# Assumptions
1. It is assumed that only deposit transactions can be disputed. However, the application is designed in such a way that withdraws can also be considered disputable in a later version without much refactoring.
2. After a charge-back transaction the client's account is frozen and future transactions are not accepted.
3. Only a deposit transaction can register a new client account.

# Tests
## Unit tests
Most of the application's logic is under ``account.rs``. The normal flow and several corner cases of this logic are tested via unit tests.

## Integration tests
The rest of the application concerns mainly the parsing of the input CSV file and the output of the registered accounts in a CSV format.
Most of this logic is also tested via the integration test.

## Code coverage
```cargo tarpaulin``` gives a code coverage of more than 90%
```
May 30 06:17:31.381  INFO cargo_tarpaulin::report: Coverage Results:
|| Uncovered Lines:
|| src/accounts.rs: 645-646
|| src/main.rs: 16-17, 19-21, 23-24, 26, 29, 31, 33
|| src/transactions.rs: 131
|| tests/integration_tests.rs: 19-20, 42-43
|| Tested/Total Lines:
|| src/accounts.rs: 224/226 +0.88%
|| src/engine.rs: 14/14 +0.00%
|| src/main.rs: 0/11 +0.00%
|| src/transactions.rs: 29/30 +0.00%
|| tests/integration_tests.rs: 26/30 +0.00%
|| 
94.21% coverage, 293/311 lines covered, +0.6430868167202561% change in coverage
```

# Error handling
The application fails and terminates only if it cannot open the given input file or if it cannot open the necessary file used for logging.