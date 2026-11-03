//! Currency-conversion aggregates embedded in statement transactions.
//!
//! `CURRENCY` and `ORIGCURRENCY` carry a currency code and conversion rate.

use crate::types::{CurrencyCode, OfxAmount};

/// Currency information attached to a transaction or balance.
///
/// OFX allows either a CURRENCY or ORIGCURRENCY element, but not both.
/// Both contain a currency code and an exchange rate.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyInfo {
    /// The transaction amount is in this currency (converted from statement default).
    Currency {
        /// Currency code of the converted amount.
        code: CurrencyCode,
        /// Exchange rate relative to the statement currency.
        rate: OfxAmount,
    },
    /// The transaction amount is in the original currency (before conversion).
    OrigCurrency {
        /// Currency code before conversion.
        code: CurrencyCode,
        /// Exchange rate relative to the statement currency.
        rate: OfxAmount,
    },
}

impl CurrencyInfo {
    /// Returns the currency code.
    #[must_use]
    pub const fn code(&self) -> &CurrencyCode {
        match self {
            Self::Currency { code, .. } | Self::OrigCurrency { code, .. } => code,
        }
    }

    /// Returns the exchange rate.
    #[must_use]
    pub const fn rate(&self) -> OfxAmount {
        match self {
            Self::Currency { rate, .. } | Self::OrigCurrency { rate, .. } => *rate,
        }
    }
}
