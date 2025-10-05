use rust_decimal::Decimal;
use serde::{Serialize, ser::SerializeStruct};

use crate::ClientId;

const DECIMAL_PLACES: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl Default for Account {
    fn default() -> Self {
        Account {
            id: 0,
            available: Decimal::new(0, DECIMAL_PLACES),
            held: Decimal::new(0, DECIMAL_PLACES),
            total: Decimal::new(0, DECIMAL_PLACES),
            locked: false,
        }
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Account", 5)?;

        state.serialize_field("client", &self.id)?;
        state.serialize_field("available", &format!("{:.4}", self.available))?;
        state.serialize_field("held", &format!("{:.4}", self.held))?;
        state.serialize_field("total", &format!("{:.4}", self.total))?;
        state.serialize_field("locked", &self.locked)?;

        state.end()
    }
}
