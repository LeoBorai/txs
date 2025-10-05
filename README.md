# Account Transaction Processor

Transaction processor for bank accounts supporting deposits, withdrawals, disputes, resolutions, and chargebacks.

## Usage

This binary expects a CSV file of the form:

```csv
type, client, tx, amount
deposit, 1, 1, 1.0
```

The CSV file path is the first argument to the binary.

```bash
cargo r -- <input.csv> > output.csv
```

The output CSV will contain account summaries, such output is
streamed to stdout,

> Some examples are available in the `fixtures` directory.

## Features

- Supports deposits, withdrawals, disputes, resolutions, and chargebacks.
- Handles multiple clients and transactions.
- Ensures account integrity with locked accounts after chargebacks.
- Stream based CSV read processing for memory efficiency.
