//! Top-level typed OFX document.
//!
//! This aggregate joins the parsed header, sign-on response, and the supported
//! banking, credit-card, and investment message sets.

use crate::header::OfxHeader;

use super::banking_message_set::BankingMessageSet;
use super::cc_message_set::CreditCardMessageSet;
use super::investment_message_set::InvestmentMessageSet;
use super::signon_response::SignonResponse;

/// A parsed OFX document, containing the header, signon response, and
/// message set responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfxDocument {
    header: OfxHeader,
    signon: SignonResponse,
    banking: Vec<BankingMessageSet>,
    credit_card: Vec<CreditCardMessageSet>,
    investment: Vec<InvestmentMessageSet>,
}

impl OfxDocument {
    /// Creates a new `OfxDocument`.
    #[must_use]
    pub const fn new(header: OfxHeader, signon: SignonResponse) -> Self {
        Self {
            header,
            signon,
            banking: Vec::new(),
            credit_card: Vec::new(),
            investment: Vec::new(),
        }
    }

    /// Sets the banking message set.
    #[must_use]
    pub fn with_banking(mut self, banking: BankingMessageSet) -> Self {
        self.banking.push(banking);
        self
    }

    /// Sets the credit card message set.
    #[must_use]
    pub fn with_credit_card(mut self, cc: CreditCardMessageSet) -> Self {
        self.credit_card.push(cc);
        self
    }

    /// Sets the investment message set.
    #[must_use]
    pub fn with_investment(mut self, investment: InvestmentMessageSet) -> Self {
        self.investment.push(investment);
        self
    }

    /// Returns the OFX header.
    #[must_use]
    pub const fn header(&self) -> &OfxHeader {
        &self.header
    }

    /// Returns the signon response.
    #[must_use]
    pub const fn signon(&self) -> &SignonResponse {
        &self.signon
    }

    /// Returns the banking message set, if present.
    #[must_use]
    pub fn banking(&self) -> Option<&BankingMessageSet> {
        self.banking.first()
    }

    /// Returns all banking message sets.
    #[must_use]
    pub fn bankings(&self) -> &[BankingMessageSet] {
        &self.banking
    }

    /// Returns the credit card message set, if present.
    #[must_use]
    pub fn credit_card(&self) -> Option<&CreditCardMessageSet> {
        self.credit_card.first()
    }

    /// Returns all credit-card message sets.
    #[must_use]
    pub fn credit_cards(&self) -> &[CreditCardMessageSet] {
        &self.credit_card
    }

    /// Returns the investment message set, if present.
    #[must_use]
    pub fn investment(&self) -> Option<&InvestmentMessageSet> {
        self.investment.first()
    }

    /// Returns all investment message sets.
    #[must_use]
    pub fn investments(&self) -> &[InvestmentMessageSet] {
        &self.investment
    }
}
