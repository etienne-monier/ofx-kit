//! Structured payee information embedded in a statement transaction.
//!
//! The model follows the supported `PAYEE` fields. OFX requires the payee name
//! but permits the address and telephone fields to be omitted.

/// A payee aggregate from an OFX transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    name: String,
    address1: Option<String>,
    address2: Option<String>,
    address3: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
    phone: Option<String>,
}

impl Payee {
    /// Creates a new payee with its required name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            address1: None,
            address2: None,
            address3: None,
            city: None,
            state: None,
            postal_code: None,
            country: None,
            phone: None,
        }
    }

    /// Sets the first address line.
    #[must_use]
    pub fn with_address1(mut self, address1: String) -> Self {
        self.address1 = Some(address1);
        self
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
    pub fn address1(&self) -> Option<&str> {
        self.address1.as_deref()
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
    pub fn city(&self) -> Option<&str> {
        self.city.as_deref()
    }

    /// Sets the city or locality.
    #[must_use]
    pub fn with_city(mut self, city: String) -> Self {
        self.city = Some(city);
        self
    }

    #[must_use]
    /// Returns the state or province.
    pub fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    /// Sets the state or province.
    #[must_use]
    pub fn with_state(mut self, state: String) -> Self {
        self.state = Some(state);
        self
    }

    #[must_use]
    /// Returns the postal code.
    pub fn postal_code(&self) -> Option<&str> {
        self.postal_code.as_deref()
    }

    /// Sets the postal code.
    #[must_use]
    pub fn with_postal_code(mut self, postal_code: String) -> Self {
        self.postal_code = Some(postal_code);
        self
    }

    #[must_use]
    /// Returns the optional country code.
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    #[must_use]
    /// Returns the telephone number.
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    /// Sets the telephone number.
    #[must_use]
    pub fn with_phone(mut self, phone: String) -> Self {
        self.phone = Some(phone);
        self
    }
}
