# Suspect Cash-Out Fund Flow to Exchange

Status: investigative — facts and confidence only, not attribution.

Value is traced as ETH/WETH (unified 1:1) plus USDC/USDT, value-conserving (taint-limited) so an intermediary's unrelated throughput is excluded. ETH-equivalent uses ETH/USD = `2500`.

## Scope

- suspect: `0x02467dd05200e5B3FBcdddbF43735aE4d0681a59`
- control contract: `0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4`
- victim vault: `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`
- token: `0xc3A640bD249381F8097f44C1b61C46172068cDff`
- drain block: `25202411`
- trace window: `25152411..=25243155`
- assets: `ETH, USDC, USDT`; depth<=`4`; min value `0.01` ETH-eq

## Result

- addresses discovered: `66`
- value movement edges: `72`
- total value out traced: `0.6934` ETH-eq
- by asset (units): `{"ETH": 0.693419301134991}`
- CEX endpoints reached: `0` (`-0.0000` ETH-eq)
- unlabeled terminal leads: `1`
- funding sources observed: `0` (CEX: `0`)

## Recoverability (ETH-eq)

Value-conserving split of traced value into actionability buckets (DESIGN §4.1).

- at an exchange (freeze-able via the exchange): `0.0000`
- in a wallet (still traceable / potentially recoverable): `0.0967`
- bridged (cross-chain follow-up): `0.0000`
- destroyed (burn): `0.0000`

## Cash-Out Endpoints (Centralized Exchanges)

No labeled CEX deposit/hot-wallet endpoint was reached within the depth budget. See unlabeled leads below for CEX-deposit follow-up.

## Funding Sources (Operation Entry Points)

No incoming value to the seeds observed in the window.

## Unlabeled Terminal Leads (next step: CEX-deposit check)

These sinks received traced value but are not in the known-address catalogs. Confirm via explorer labels whether any is a CEX deposit address; if so, add it to `reth_chain_query/common_addresses/cex` so future traces resolve it automatically.

| Address | Depth | Value Received (ETH-eq) |
| --- | ---: | ---: |
| `0xdF01abaaB10167b86385AdD3FD7Ae83B4E35839A` | `2` | `0.0967` |

## Artifacts

- `artifacts/tx_fund_flow/suspect_cashout_trace.json`
