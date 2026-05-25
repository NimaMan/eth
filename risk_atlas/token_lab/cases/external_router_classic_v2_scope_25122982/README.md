# External-Router Sell Scope For Classic V2 Strategy Routes

## Scope

Risk Atlas run: `risk-atlas-uniswap-v2-20k-20260520`

Blocks reviewed:

- `25,122,982`: `Compass` / `0xc38a7a4b93bd2112a021c1b55756e5d1805cd237`
- `25,129,615`: `COMPASS AI` / `0x47a71baffde69472b60031f79361e556964aaf61`

This investigation uses current reproducible Risk Atlas rows under
`observed_sell_via_external_router`.

## Question

If chain activity shows an economically meaningful sell through an external
router, should Alpha treat the pool as sellable?

## Findings

There are two different mechanisms in this review. They should not be collapsed
into one generic `observed_sell_via_external_router` bucket.

### `Compass`: True Chain-Parity Route Case

This is the primary chain-parity issue.

The pool was not drained. At block `25,122,982`, the pool still had
`1.8794457585863258 WETH`, and three same-block external-router sells extracted
`0.8245830985731981 WETH`.

Pre-block tests using the actual sellers and actual token balances still fail
through Alpha's classic Uniswap V2 route with
`TransferHelper: TRANSFER_FROM_FAILED`. The successful chain txs use helper
routes instead:

- `0x80a64c...` selector `0x3d0e3ec5`, decoded as
  `swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256,address)`;
- `0xF82772...` selector `0x08c1284c`, decoded as `SigmaSwap(bytes)`;
- `0x3328F7...` selector `0x75713a08`, unknown selector but trace shows a
  helper route.

The trace shows why the route matters. During `Compass.transferFrom`, the token
performs a token-contract self-swap through the Uniswap V2 router before the
holder transfer completes. The helper routes call `token.transferFrom` first,
allow that internal self-swap to happen, then compute and call `pair.swap`
directly. Alpha's current classic V2 sell path uses the Uniswap V2 router as
the top-level path, so it is not execution-equivalent for this token class.

This is a source/simulator parity gap until Risk Atlas can replay or classify
the observed route shape:

- `chain_route_sellable=true`: the mined helper route can sell from a liquid
  pool.
- `classic_router_route_sellable=false`: the classic V2 router route fails.
- `alpha_v2_vault_route_sellable=false_until_historical_vault_rehearsal`: the
  deployed V2 vault also calls the Uniswap V2 router internally, so it is not
  equivalent to the observed helper routes.

The deployed V2 vault at `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` was not
available at the historical blocks in this investigation. `eth_getCode`
returns no code at blocks `25,122,981`, `25,122,982`, and `25,129,615`; the
same address has deployed runtime code only at the current local head. That
means an exact historical replay through the deployed address requires a
simulator overlay that injects or deploys the vault runtime into the historical
fork.

The route-shape approximation is already enough to avoid a false positive:
using the vault address as the seller while keeping Alpha's classic V2 route
still fails for `Compass` at block `25,122,982` with
`TransferHelper: TRANSFER_FROM_FAILED` at every tested size from `100%` down to
`1%`. This matches the contract shape of
`emergencySellV2ExactTokensForEth`, which approves the Uniswap V2 router and
then calls the router's fee-on-transfer sell path.

### `COMPASS AI`: Stateful Fresh-Buyer Restriction

This is not the same issue.

For `COMPASS AI`, sell-only simulation succeeds through the classic Uniswap V2
route, including with Alpha's default synthetic seller. But full
`buy -> approve -> sell` fails after the buy. That points to stateful
fresh-holder restriction: the buy changes holder/token state in a way that makes
the immediate sell fail. The observed sell is therefore not enough evidence that
a freshly bought Alpha position can exit.

The exact deployed vault also reproduces this at current local head: vault
`buyV2ExactEthForTokens` succeeds, then
`emergencySellV2ExactTokensForEth` reverts with
`TransferHelper: TRANSFER_FROM_FAILED` when the Uniswap V2 router calls
`token.transferFrom(vault, pool, amount)`.

Risk Atlas should classify this separately from helper-route parity.

## Risk Atlas Evidence

Current DB rows for `risk-atlas-uniswap-v2-20k-20260520`:

| Token | Pool | Block | Failure Class | Observed Sells | WETH Out | Market Flags |
| --- | --- | ---: | --- | ---: | ---: | --- |
| `0x47a71b...aaf61` | `0x7f4e96...06138` | 25,129,615 | `observed_sell_via_external_router` | 1 | `0.01748963470624494` | `can_buy=true`, `can_sell=false` |
| `0xc38a7a...5cd237` | `0x0d3d3a...3e30` | 25,122,982 | `observed_sell_via_external_router` | 3 | `0.8245830985731981` | `can_buy=true`, `can_sell=false` |

This class should only be used when pool liquidity is meaningful. If the pool is
already depleted, or the only observed sell output is dust, the right
classification is amount-quality/dead-pool behavior, not route-scope mismatch.

| Token | Block | WETH Reserve | Token Reserve | Price / Initial |
| --- | ---: | ---: | ---: | ---: |
| `0x47a71b...aaf61` | 25,129,615 | `1.007852162168755` | `78,245,497.89995359` | `1.0156385366557106x` |
| `0xc38a7a...5cd237` | 25,122,981 | `2.1100288571595236` | `179,562,160.71626365` | `2.6278503056677844x` |
| `0xc38a7a...5cd237` | 25,122,982 | `1.8794457585863258` | `201,954,775.40616047` | `2.225015400720721x` |
| `0xc38a7a...5cd237` | 25,122,983 | `2.745550119029709` | `138,422,951.1607484` | `3.522633228635679x` |

Representative mined external-router transactions:

| Token | Tx | Block | Tx Index | Router/Target | WETH Out |
| --- | --- | ---: | ---: | --- | ---: |
| `COMPASS AI` | `0x1ae5b7...04b8b14` | 25,129,615 | 7 | `0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e` | `0.01748963470624494` |
| `Compass` | `0x23b55d...937e87` | 25,122,982 | 5 | `0xF827725498E6fcF62D331566965F5254bCda081f` | `0.10934270432841361` |
| `Compass` | `0x3cdcb5...a98580` | 25,122,982 | 98 | `0x3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49` | `0.3268768987896309` |
| `Compass` | `0x4b4d99...4dad2` | 25,122,982 | 93 | `0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e` | `0.3883634954551535` |

`CLARITY` also appears under the same failure class in the current run, but its
observed sell output is dust. It belongs under amount-quality/dust review, not
this route-scope investigation.

## Deployed V2 Vault Probe Results

Vault: `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`.

Owner: `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27`.

| Probe | Coordinate | Result | Interpretation |
| --- | --- | --- | --- |
| Vault code existence | blocks `25,122,981`, `25,122,982`, `25,129,615` | no code | Exact deployed-address historical replay is not possible without a simulator code/deploy overlay. |
| `Compass` sell, seller set to vault, classic V2 route | block `25,122,982` | all tested sizes fail with `TransferHelper: TRANSFER_FROM_FAILED` | The deployed vault's route shape should not be treated as equivalent to helper-router sells. |
| Exact deployed vault `buy -> sell`, `Compass` | current local head `25,167,381` | buy succeeds; vault sell succeeds | Useful smoke test that the deployed vault can work for some current-state tokens, but not historical evidence for block `25,122,982`. |
| Exact deployed vault `buy -> sell`, `COMPASS AI` | current local head `25,167,381` | buy succeeds; vault sell fails with `TransferHelper: TRANSFER_FROM_FAILED` | Confirms the deployed vault can hit the same fresh-holder sell restriction. |

## Strategy-Route Probe Results

Command shape:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
cargo run -q -p tx_processor --example probe_pool_buy_sell -- \
  --token <TOKEN> \
  --pool <POOL> \
  --protocol uniswap-v2 \
  --block <BLOCK> \
  --amount-wei 10000000000000000,1000000000000000 \
  --token-decimals 9 \
  --reth-datadir /home/nima/storage/samsung8tb/ethereum/reth
```

Results:

| Token | Block Checked | Amount | Buy | Approve | Sell | Failure |
| --- | ---: | ---: | --- | --- | --- | --- |
| `0xc38a7a...5cd237` | 25,122,981 | `0.01 ETH` | true | true | false | `TransferHelper: TRANSFER_FROM_FAILED` |
| `0xc38a7a...5cd237` | 25,122,981 | `0.001 ETH` | true | true | false | `TransferHelper: TRANSFER_FROM_FAILED` |
| `0xc38a7a...5cd237` | 25,122,982 | `0.01 ETH` | true | true | false | `TransferHelper: TRANSFER_FROM_FAILED` |
| `0xc38a7a...5cd237` | 25,122,982 | `0.001 ETH` | true | true | false | `TransferHelper: TRANSFER_FROM_FAILED` |

Additional `COMPASS AI` checks:

- sell-only through classic Uniswap V2 succeeds for the default synthetic seller;
- sell-only through classic Uniswap V2 succeeds for the observed seller;
- full `buy -> approve -> sell` fails after the buy.

## Policy Consequence

For current Alpha V2 policies:

- keep `can_sell=false` for classic V2 route probes;
- keep `effective_can_sell=false` for executable Alpha strategy eligibility;
- expose observed helper-route sells as source evidence with route scope, not as
  generic strategy sellability;
- add tx-index-aware observed-route replay before treating helper-route sells as
  parity evidence for any Alpha route;
- add exact historical V2 vault rehearsal before setting
  `alpha_v2_vault_route_sellable=true`; until then, a helper-route sell is not
  Alpha-executable sellability;
- treat external/helper-router support as a separate future route
  implementation with its own decoder, calldata builder, simulation, gas
  policy, and live execution validation.

## Follow-Up

Risk Atlas should make route scope explicit anywhere sellability is displayed or
exported:

- source observation route: `classic_v2`, `universal_router`, `permit2`,
  `external_router`, or `unknown`;
- executable route: the exact route Alpha can build and simulate;
- replay coordinate: block plus transaction index, not only block number, for
  same-block helper-route sells;
- amount quality: separate non-dust external sells from dust-only movement;
- policy label: `external_route_only_sell_observed` rather than generic
  sellable.

Implementation steps:

1. Add a historical V2 vault rehearsal path in `tx_simulator` that can deploy or
   inject the V2 vault into a historical fork, then run the exact
   `buyV2ExactEthForTokens` and `emergencySellV2ExactTokensForEth` calls.
2. Add tx-index-aware observed-route replay: start from block `N-1`, replay the
   signed block prefix up to the observed helper-route tx index, replay the
   observed helper tx, then replay Alpha's executable route at the same
   coordinate.
3. Persist route-specific fields in Risk Atlas:
   `observed_sell_route_class`, `observed_sell_tx_index`,
   `observed_sell_weth_out`, `classic_router_route_sellable`,
   `alpha_v2_vault_route_sellable`, and `route_parity_status`.
4. Keep strategy policy conservative: helper-route-only evidence is research
   signal until Alpha has an executable helper route or the exact vault route
   passes at the same historical coordinate.
