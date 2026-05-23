# Token Builder Replay - 2026-05-20

Local server-backed token-builder replay of the SSS investigation window. Values
below are the range-end snapshot at block `25041137`, not the current/latest
market state shown by Dexscreener.

## Run

- API: `POST /api/v1/eth/ranges`
- request: `{"start_block":25041048,"end_block":25041137,"history_limit":1000,"retention_mode":"keep_all"}`
- local run id: `run-1`
- blocks: `25041048..25041137` (`90` blocks)
- source: `processed_block_disk_cache`
- cache hits/misses: `90/0`
- txs scanned/processed: `23015/22729`
- token update reports: `827`
- transaction failures: `0`
- pool simulation failures: `0`

## Token Snapshot

- token: `0x144742cc48cacb0f2e49dd03af5cfca3fa30e7bc`
- name/symbol: `SpaceX space services` / `SSS`
- decimals: `9`
- total supply: `1000000000.0`
- creator: `0xd962715b742d7a761a09a5141d85679e8472a6b0`
- ownership: renounced
- behavior flags: `cannot_sell_pool`
- token-builder evidence: `1 pool(s) can buy but cannot sell`

## Pool Snapshot

- pool: `0x06f53d7c71f91060d7e698218d78ff293f4f2aa5`
- protocol: `UNISWAP-V2`
- denom: `WETH`
- range-end token reserve: `697.712149384`
- range-end WETH reserve: `1.4850086199025743`
- raw price ratio to initial: `2128397.249802325`
- pooled token supply percent: `0.0000697712149384`
- `can_buy=true`, `can_sell=false`
- `effective_can_buy=true`, `effective_can_sell=false`
- `has_observed_buy=true`, `has_observed_sell=true`
- stage: `CANNOT_SELL`
- risk: `honeypot` / `cannot_sell`
- latest sell check: `TransferHelper: TRANSFER_FROM_FAILED`
- latest failure class: `observed_buy_via_external_router`

## Key Transaction Rows

| Label | Block | Token builder row |
| --- | ---: | --- |
| `initial_liquidity` | 25041048 | `1,000,000,000` SSS transfer and `1` WETH transfer; no buy/sell volume. |
| `first_tracked_buy` | 25041049 | Buy volume `0.198` WETH; token transfer volume `164,861,375.33969265` SSS. |
| `creator_sync_after_reserve_collapse` | 25041123 | Creator tx with zero token transfers and zero denom transfers. |
| `weth_drain_sell` | 25041123 | Sell volume `8.594034272473914` WETH; token transfer volume `119,838,850.44120963` SSS. |
| `later_buy` | 25041137 | Buy volume `1.485` WETH; token transfer volume `119,838,272.56803052` SSS. |

## Interpretation

The token builder reproduces the issue. It records the direct creator `sync`
with no token or denom transfers, the same-block WETH-drain sell, and the later
buy into a dust-token reserve state. The range-end pool read model is
internally consistent with the chain-truth reserve table: WETH reserve is
non-zero, token reserve is dust relative to total supply, and the raw price
ratio is therefore huge but unsafe.

This replay does not prove `observed_sell_simulator_fail`: the range builder had
zero pool simulation failures and the successful WETH drain is observed from
executed chain data. The remaining issue is detector/display policy: the pool
should surface reserve-quality danger instead of presenting the high ratio as a
normal winner, and it may belong under `privileged_seller_reserve_drain` if a
same-prestate sell check confirms retail/fresh-holder sell blocking while the
drain actor can sell.
