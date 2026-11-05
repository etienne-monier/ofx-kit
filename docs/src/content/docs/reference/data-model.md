---
title: Data model
description: Navigate the typed document returned by ofx-kit.
---

## Document shape

The parser returns an [`OfxDocument`](https://docs.rs/ofx-kit/latest/ofx_kit/struct.OfxDocument.html).
The following diagram shows the usual navigation path for a bank statement:

```text
OfxDocument
├── header() -> OfxHeader
├── signon() -> SignonResponse
│   └── status(), date_time_server(), language(), fi_org(), fi_id()
├── banking() / bankings() -> BankingMessageSet
│   └── statement_responses()
│       └── TransactionWrapper<StatementResponse>
│           ├── status()
│           └── response()
│               ├── currency_default() -> CurrencyCode
│               ├── bank_account() -> BankAccount
│               ├── transaction_list() -> Option<TransactionList>
│               │   └── transactions() -> &[StatementTransaction]
│               ├── ledger_balance() -> Option<LedgerBalance>
│               └── available_balance() -> Option<AvailableBalance>
├── credit_card() / credit_cards() -> CreditCardMessageSet
└── investment() / investments() -> InvestmentMessageSet
```

Credit-card statements follow the same balance and transaction structure but
contain a `CreditCardAccount`. Investment statements are intentionally partial:
the typed support currently extracts account metadata and banking transactions
from `INVBANKTRAN` / `STMTTRN`, not investment positions or trades.

## Optional values and repeated sets

OFX providers do not all return the same optional fields. The model exposes
optional data as `Option<&T>` and collections as slices, so callers should not
assume that a transaction list or balance exists.

Documents may contain repeated top-level message sets. The singular accessors
(`banking()`, `credit_card()`, and `investment()`) return the first set. The
plural accessors preserve and expose all sets:

```rust
for banking in document.bankings() {
    for wrapper in banking.statement_responses() {
        if let Some(statement) = wrapper.response()
            && let Some(transactions) = statement.transaction_list()
        {
            println!("{} transactions", transactions.len());
        }
    }
}
```

## Transaction values

`StatementTransaction` provides typed values for the fields most useful when
processing statements:

| Method | Meaning |
| --- | --- |
| `transaction_type()` | OFX transaction classification |
| `date_posted()` | Posting date/time |
| `amount()` | Exact signed amount as `OfxAmount` |
| `fit_id()` | Provider transaction identifier |
| `name()` / `memo()` | Human-readable transaction text |
| `check_number()` / `reference_number()` | Provider reference values |
| `payee()` | Optional structured payee |
| `currency()` | Optional transaction currency information |

The API deliberately uses accessors instead of exposing mutable fields. This
keeps validation at the parsing boundary and makes missing provider data
explicit.
