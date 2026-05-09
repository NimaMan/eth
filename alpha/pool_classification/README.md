# Pool Classification

Central Rust crate for pool-level tradability classification.

This crate owns the shared category contract used by:

- token-server pool views and table buckets;
- historical launch-strategy stats;
- Snipe All entry rules;
- token-lab detector parity checks.

The first split is `eligible` versus `ineligible`. Eligible pools can then be
classified as `eligible_active` or `eligible_risk` based on the current outcome.
This lets historical analysis keep pools that were tradable at entry even if
they later lost liquidity, became unsellable, showed hidden mint evidence, or
hit high-tax/risk outcomes.

Run the crate tests with:

```text
cargo test -p eth_pool_classification
```

The JSON CLI accepts a single pool object, a JSON array, or an object with a
`pools` array:

```text
cargo run -q -p eth_pool_classification --bin pool_classification <<'JSON'
{"currency":"WETH","denom_reserve":0.5,"can_buy":true,"can_sell":true}
JSON
```
