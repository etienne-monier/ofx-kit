//! OFX three-letter currency-code value.

use core::fmt;
use core::str::FromStr;

/// An ISO-4217 currency code (exactly 3 uppercase ASCII letters).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Returns the currency code as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Error returned when parsing an invalid currency code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCurrencyCode {
    /// The input string was empty.
    #[error("currency code cannot be empty")]
    Empty,
    /// The input string was not exactly 3 characters.
    #[error("currency code must be exactly 3 characters, got {length}")]
    WrongLength {
        /// Number of characters received.
        length: usize,
    },
    /// The input string contained a non-uppercase-ASCII character.
    #[error("currency code contains invalid character '{character}' at position {position}")]
    InvalidCharacter {
        /// Rejected character.
        character: char,
        /// Character position in the input.
        position: usize,
    },
}

impl FromStr for CurrencyCode {
    type Err = InvalidCurrencyCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(InvalidCurrencyCode::Empty);
        }
        if s.len() != 3 {
            return Err(InvalidCurrencyCode::WrongLength { length: s.len() });
        }
        for (i, c) in s.chars().enumerate() {
            if !c.is_ascii_uppercase() {
                return Err(InvalidCurrencyCode::InvalidCharacter {
                    character: c,
                    position: i,
                });
            }
        }
        Ok(Self(s.to_owned()))
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for CurrencyCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_usd_succeeds() {
        let code: CurrencyCode = "USD".parse().unwrap();
        assert_eq!(code.as_str(), "USD");
    }

    #[test]
    fn parse_eur_succeeds() {
        assert!("EUR".parse::<CurrencyCode>().is_ok());
    }

    #[test]
    fn parse_gbp_succeeds() {
        assert!("GBP".parse::<CurrencyCode>().is_ok());
    }

    #[test]
    fn parse_lowercase_returns_error() {
        let err = "usd".parse::<CurrencyCode>().unwrap_err();
        assert!(matches!(err, InvalidCurrencyCode::InvalidCharacter { .. }));
    }

    #[test]
    fn parse_two_chars_returns_wrong_length() {
        let err = "US".parse::<CurrencyCode>().unwrap_err();
        assert!(matches!(
            err,
            InvalidCurrencyCode::WrongLength { length: 2 }
        ));
    }

    #[test]
    fn parse_four_chars_returns_wrong_length() {
        let err = "USDD".parse::<CurrencyCode>().unwrap_err();
        assert!(matches!(
            err,
            InvalidCurrencyCode::WrongLength { length: 4 }
        ));
    }

    #[test]
    fn parse_with_digit_returns_invalid_character() {
        let err = "US1".parse::<CurrencyCode>().unwrap_err();
        assert!(matches!(
            err,
            InvalidCurrencyCode::InvalidCharacter {
                character: '1',
                position: 2
            }
        ));
    }

    #[test]
    fn parse_empty_returns_error() {
        let err = "".parse::<CurrencyCode>().unwrap_err();
        assert!(matches!(err, InvalidCurrencyCode::Empty));
    }

    #[test]
    fn display_roundtrip() {
        let code: CurrencyCode = "JPY".parse().unwrap();
        assert_eq!(code.to_string().parse::<CurrencyCode>().unwrap(), code);
    }
}
