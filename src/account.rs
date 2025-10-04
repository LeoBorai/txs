use rust_decimal::Decimal;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Account {
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
    pub total: Decimal,
}
