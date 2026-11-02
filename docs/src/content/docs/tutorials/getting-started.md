---
title: Getting started
description: Parse your first OFX document with ofx-kit.
---

## Add the dependency

```sh
cargo add ofx-kit
```

## Parse a document

```rust
use ofx_kit::parse;

let content = std::fs::read_to_string("statement.ofx")?;
let document = parse(&content)?;

assert!(document.signon().status().is_success());
```

The parser accepts both OFX 1.x SGML and OFX 2.x XML through the same `parse`
function. The library itself does not perform file I/O.
