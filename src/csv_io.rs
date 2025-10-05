use std::io::stdout;
use std::path::Path;
use std::task::{Context, Poll};
use std::{fs::File, pin::Pin};

use anyhow::Result;
use csv::{ReaderBuilder, Trim};
use futures::Stream;

use crate::{account::Account, tx::Transaction};

pub struct CsvReader {
    reader: csv::Reader<File>,
}

impl CsvReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = ReaderBuilder::new()
            .trim(Trim::All)
            .from_path(path)
            .expect("Failed to build CSV reader");

        Ok(CsvReader { reader })
    }
}

impl Stream for CsvReader {
    type Item = Result<Transaction>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut iter = self.get_mut().reader.deserialize();

        match iter.next() {
            Some(result) => Poll::Ready(Some(result.map_err(|e| e.into()))),
            None => Poll::Ready(None),
        }
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
            .expect("Failed to serialize account record");
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().expect("Failed to flush writer");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use futures::TryStreamExt;

    use super::*;

    #[tokio::test]
    async fn reads_from_csv_file() -> Result<()> {
        let reader = CsvReader::new("fixtures/sample_01.csv")?;
        let txs = reader.into_stream();
        let txs: Vec<_> = txs.try_collect().await?;

        assert_eq!(txs.len(), 5);

        Ok(())
    }
}
