use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use csv::{ReaderBuilder, Trim};

use crate::tx::Transaction;

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

    pub fn read(&mut self) -> Vec<Transaction> {
        self.reader
            .deserialize()
            .map(|result| result.expect("Failed to deserialize"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn reads_from_csv_file() -> Result<()> {
        let mut reader = CsvReader::new("fixtures/sample_01.csv")?;
        let txs: Vec<Transaction> = reader.read();

        assert_eq!(txs.len(), 5);

        Ok(())
    }
}
