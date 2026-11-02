---
title: Inspect an OFX statement
description: Inspect accounts and transactions from the command line.
---

Run the inspector with a statement file:

```sh
ofx inspect statement.ofx
```

For scripts, select JSON output:

```sh
ofx inspect statement.ofx --json
```

Filter by account and date, and limit the displayed transactions per
statement:

```sh
ofx inspect statement.ofx --account 987654321 \
  --from 2023-01-01 --to 2023-01-31 --limit 20
```
