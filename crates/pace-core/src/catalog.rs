//! The catalog: the roles, clients and project records that rules, reports
//! and bills take as a value, with their money and rounding.

mod money;

pub use money::{Currency, Money, Totals};
