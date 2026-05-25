# Investigation Cases

Concrete token and pool investigations live here. Each case should be small
enough to reproduce and specific enough to become a regression test, detector
candidate, display rule, or Alpha lab input.

## Folder Contract

Use stable case folder names:

```text
<token_symbol_or_name>_<short_context>_<start_block>
```

Each case normally contains:

- `README.md`: the narrative, chain truth, simulator parity, conclusion, and
  follow-up actions;
- `investigation.toml`: machine-readable coordinates and status when useful;
- `artifacts/README.md`: instructions for generated artifacts.

Generated artifacts stay under `artifacts/` and should not become frontend or
strategy inputs directly. Promote durable facts into the owning production
module before depending on them.
