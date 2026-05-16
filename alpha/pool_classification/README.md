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

## Eligibility Contract

Eligibility is a cohort-membership decision, not a current-health decision. It
is not defined at pool creation. A pool becomes eligible at the first block in
its lifetime where the pool's current as-of-block state satisfies the first-pass
tradability rules:

- quote/denom is supported: `ETH`, `WETH`, `USDC`, `USDT`, or `DAI`;
- denom liquidity reaches the configured floor:
  - `0.5` for `ETH`/`WETH` pools;
  - `1000` for stable-denom pools;
- the cohort buy path has been observed as available;
- the cohort sell path has been observed as available;
- optional strategy-stat checks may also require creation data and price
  history.

Once a pool enters the eligible cohort, it should not become ineligible later.
Later states are eligible outcomes, not exclusion reasons:

- liquidity removed;
- current liquidity below the entry floor;
- cannot buy;
- cannot sell;
- honeypot behavior;
- hidden mint evidence;
- extreme tax;
- LP approval exposure;
- concentrated LP ownership.

The implementation uses `max_denom_reserve` as `eligibility_liquidity` when it
is available, so a pool that once crossed the liquidity floor stays in the
eligible cohort even if current liquidity later drops. It also accepts
`cohort_can_buy` and `cohort_can_sell` so current buy/sell failure can become an
eligible-risk outcome instead of retroactively removing the pool from the
cohort.

The code keeps entry detection separate from later sticky classification:

- `first_eligible_observation` scans pool observations by block and returns the
  first block where the current observation satisfies eligibility;
- `PoolClassificationInput::for_cohort_entry` ignores lifetime/sticky fields
  like `max_denom_reserve`, `cohort_can_buy`, and `cohort_can_sell` while
  testing the entry block;
- `classify_pool` can still use `max_denom_reserve`, `cohort_can_buy`, and
  `cohort_can_sell` for later snapshots after the pool has entered the cohort.

The useful launch moment for analysis is therefore the first block where the
pool becomes tradable with supported-denom liquidity above the floor. That can
differ from the pool-creation block when liquidity is added, trading opens, or
buy/sell simulation becomes observable a few blocks later.

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
