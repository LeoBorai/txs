use std::fs::File;
use std::io::stdout;
use std::path::Path;

use anyhow::{Context, Result};
use csv::{ReaderBuilder, Trim};

use crate::{account::Account, tx::Transaction};

pub struct CsvReader {
    reader: csv::Reader<File>,
}

impl CsvReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = ReaderBuilder::new()
            .trim(Trim::All)
            .from_path(path)
            .context("Failed to build CSV reader")?;

        Ok(CsvReader { reader })
    }

    pub fn load_in_memory(&mut self) -> Vec<Transaction> {
        self.reader
            .deserialize()
            .map(|result| result.expect("Failed to deserialize"))
            .collect()
    }
}

pub struct CsvWriter {
    writer: csv::Writer<std::io::Stdout>,
}

impl CsvWriter {
    pub fn new() -> Result<Self> {
        let writer = csv::Writer::from_writer(stdout());

        Ok(CsvWriter { writer })
    }

    pub fn write(&mut self, record: &Account) -> Result<()> {
        self.writer
            .serialize(record)
            .context("Failed to serialize account record")?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().context("Failed to flush writer")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn reads_from_csv_file() -> Result<()> {
        let mut reader = CsvReader::new("fixtures/sample_01.csv")?;
        let txs: Vec<Transaction> = reader.load_in_memory();

        assert_eq!(txs.len(), 5);

        Ok(())
    }
}
