---
title: Use the CLI
description: Validate, inspect, and decode OFX files.
---

Install and run the `ofx` binary, then validate a file before importing it:

```sh
ofx validate statement.ofx
```

Use JSON when integrating with another tool:

```sh
ofx validate statement.ofx --json
ofx inspect statement.ofx --json > statement.json
```

For legacy files, `decode` converts the detected source encoding to UTF-8:

```sh
ofx decode legacy.ofx > legacy-utf8.ofx
```
