mod account;
mod csv_io;
mod error;
mod ledger;
mod tx;

use std::env::args;
use std::fs::OpenOptions;

use anyhow::{Result, bail};
use futures::{StreamExt, TryStreamExt};
use tracing::{Level, error};
use tracing_subscriber::FmtSubscriber;

use crate::csv_io::{CsvReader, CsvWriter};
use crate::ledger::Ledger;

pub type ClientId = u16;
pub type TransactionId = u32;

#[tokio::main]
async fn main() -> Result<()> {
    let err_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("error.log")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .with_ansi(false)
        .with_writer(err_log)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

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
                error!("Error reading transaction: {:?}", e);
                continue;
            }
            Ok(tx) => {
                if let Err(e) = ledger.process_tx(tx) {
                    error!("Error processing transaction {:?}", e);
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
