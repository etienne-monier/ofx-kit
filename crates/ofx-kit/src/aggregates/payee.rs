//! Structured payee information embedded in a statement transaction.
//!
//! The model follows the currently supported `PAYEE` fields and is constructed
//! only after the parser has validated the required fields it supports.

/// A payee aggregate from an OFX transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    name: String,
    address1: String,
    address2: Option<String>,
    address3: Option<String>,
    city: String,
    state: String,
    postal_code: String,
    country: Option<String>,
    phone: String,
}

impl Payee {
    /// Creates a new `Payee` with required fields.
    #[must_use]
    pub const fn new(
        name: String,
        address1: String,
        city: String,
        state: String,
        postal_code: String,
        phone: String,
    ) -> Self {
        Self {
            name,
            address1,
            address2: None,
            address3: None,
            city,
            state,
            postal_code,
            country: None,
            phone,
        }
    }

    /// Sets the optional second address line.
    #[must_use]
    pub fn with_address2(mut self, address2: String) -> Self {
        self.address2 = Some(address2);
        self
    }

    /// Sets the optional third address line.
    #[must_use]
    pub fn with_address3(mut self, address3: String) -> Self {
        self.address3 = Some(address3);
        self
    }

    /// Sets the optional country code.
    #[must_use]
    pub fn with_country(mut self, country: String) -> Self {
        self.country = Some(country);
        self
    }

    #[must_use]
    /// Returns the payee name.
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Returns the first address line.
    pub fn address1(&self) -> &str {
        &self.address1
    }

    #[must_use]
    /// Returns the optional second address line.
    pub fn address2(&self) -> Option<&str> {
        self.address2.as_deref()
    }

    #[must_use]
    /// Returns the optional third address line.
    pub fn address3(&self) -> Option<&str> {
        self.address3.as_deref()
    }

    #[must_use]
    /// Returns the city.
    pub fn city(&self) -> &str {
        &self.city
    }

    #[must_use]
    /// Returns the state or province.
    pub fn state(&self) -> &str {
        &self.state
    }

    #[must_use]
    /// Returns the postal code.
    pub fn postal_code(&self) -> &str {
        &self.postal_code
    }

    #[must_use]
    /// Returns the optional country code.
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    #[must_use]
    /// Returns the telephone number.
    pub fn phone(&self) -> &str {
        &self.phone
    }
}
