# Stable-Denom Execution Scope

## Scope

Historical source range: `25,065,694..25,080,693`

Previous run: `hist-snipe-all-poolonly-15k-v3-ur-decoded-20260513-132029`

Fixed rerun: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

This investigation covers the stable-denom entries that appeared in the 15k
historical baseline while the strategy budget and PnL accounting are still
ETH-denominated.

## Finding

The prior baseline allowed USDC/USDT-denominated V2 pools through entry
eligibility even though the executable strategy amount is a single
18-decimal `buy_amount_wei`.

That created two distinct parity problems:

- stable-denom failed buys were reported as router empty reverts because the
  direct V2/V3 buy path is ETH/WETH-oriented;
- one USDT-denom position confirmed, but it was not a clean same-pool,
  ETH-equivalent entry/exit because cost basis and sell proceeds are recorded
  as 18-decimal ETH amounts.

Denom-aware pool probes showed the pools themselves were not fundamentally
untradeable. They succeeded with sane 6-decimal quote sizes and failed only
when `10000000000000000` was interpreted as raw USDC/USDT.

## Probe Results

| Token | Block | Denom | Result |
| --- | ---: | --- | --- |
| `0x87Bd42...3D19bc` | 25,070,469 | USDC | `1e16` raw USDC top-up is unavailable; `10000`, `1000`, and `100` raw USDC all buy and sell |
| `0xdEe889...Fed116` | 25,079,195 | USDT | `1e16` raw USDT top-up is unavailable; `10000`, `1000`, and `100` raw USDT all buy and sell at the entry block |

The USDT pool did not exist at `block - 1`, then became available at the entry
block. That is expected for the post-block historical semantics.

## Fix

The executable entry gate now rejects non-native/non-WETH quote pools with
`unsupported_execution_denom`, and rejects missing denomination addresses with
`unsupported_execution_missing_denom`.

This keeps stable-denom pools in Risk Atlas research classification, but keeps
them out of the executable historical baseline until we add:

- ETH-to-quote conversion or multi-hop entry support;
- quote-denom cost basis and sell proceeds normalized back to ETH;
- strategy sizing that respects each quote token's decimals.

## Verification

The fixed rerun `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`
processed the same 22,584 replay events and produced:

- 554 execution reports: 523 confirmed, 31 failed;
- 449 positions: 345 buy-confirmed, 85 sell-confirmed, 11 buy-failed, 8 sell-failed;
- no confirmed non-WETH-denominated positions;
- no empty-router buy failures;
- total PnL `+9.693440469949935980 ETH`.

## Probe Command Shape

```bash
cargo run -q -p tx_processor --example probe_pool_buy_sell -- \
  --token <TOKEN> \
  --pool <POOL> \
  --protocol uniswap-v2 \
  --block <BLOCK> \
  --denom <USDC_OR_USDT> \
  --denom-decimals 6 \
  --token-decimals 18 \
  --amount-wei 10000000000000000,10000,1000,100
```
