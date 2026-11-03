//! Available-balance aggregate from a bank or credit-card statement.
//!
//! This module models the amount and timestamp reported by `AVAILBAL`.

use crate::types::{OfxAmount, OfxDateTime};

/// An available balance (AVAILBAL) -- the amount available for withdrawal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableBalance {
    amount: OfxAmount,
    as_of: OfxDateTime,
}

impl AvailableBalance {
    /// Creates a new `AvailableBalance`.
    #[must_use]
    pub const fn new(amount: OfxAmount, as_of: OfxDateTime) -> Self {
        Self { amount, as_of }
    }

    /// Returns the balance amount.
    #[must_use]
    pub const fn amount(&self) -> OfxAmount {
        self.amount
    }

    /// Returns the date/time the balance was effective.
    #[must_use]
    pub const fn as_of(&self) -> &OfxDateTime {
        &self.as_of
    }
}
