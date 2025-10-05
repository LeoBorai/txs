mod account;
mod csv_io;
mod error;
mod ledger;
mod tx;

use std::env::args;

use anyhow::{Result, bail};
use futures::{StreamExt, TryStreamExt};

use crate::csv_io::{CsvReader, CsvWriter};
use crate::ledger::Ledger;

pub type ClientId = u16;
pub type TransactionId = u32;

#[tokio::main]
async fn main() -> Result<()> {
    let args = args().collect::<Vec<String>>();

    if args.len() != 2 {
        bail!("Usage: {} <input.csv>", args[0]);
    }

    let input_path = &args[1];

    let csv_reader = CsvReader::new(input_path)?;
    let mut csv_stream = csv_reader.into_stream();

    let mut ledger = Ledger::new();

    while let Some(mb_tx) = csv_stream.next().await {
        match mb_tx {
            Err(e) => {
                eprintln!("Error reading transaction: {:?}", e);
                continue;
            }
            Ok(tx) => {
                if let Err(e) = ledger.process_tx(tx) {
                    eprintln!("Error processing transaction {:?}", e);
                }
            }
        }
    }

    let mut csv_writer = CsvWriter::new()?;
    let accounts = ledger.accounts_summary();

    for acct in accounts.into_iter() {
        csv_writer.write(&acct)?;
    }

    csv_writer.flush()?;

    Ok(())
}
