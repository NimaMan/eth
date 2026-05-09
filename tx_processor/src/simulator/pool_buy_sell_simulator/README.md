# Pool Buy/Sell Simulator

The pool buy/sell simulator sits at the heart of the `tx_processor` trading
regression suite. Its job is to answer a simple question for any supported AMM
pool: **“Can our default hot wallet buy the token, approve it for resale, and
then successfully sell it back to the denomination token at a specific block?”**

Even though the question is simple, getting a deterministic yes/no answer is
tricky because the execution environment has to perfectly mimic on-chain
semantics. This document captures the simulator’s objectives, the execution
pipeline, and the builder choices that must be respected to avoid the common
`TransferHelper: TRANSFER_FROM_FAILED` failure we see when fee-on-transfer tokens
are handled incorrectly.

---

## High-level objectives

- Recreate a three-step trading workflow inside a forked EVM state:
  1. **Denomination preparation** – wrap ETH into WETH (if needed) and approve
     the router/pool for the denomination token.
  2. **Buy leg** – swap the denomination token (typically WETH, USDC, etc.) into
     the target token.
  3. **Approve leg** – allow the router to pull the purchased tokens.
  4. **Sell leg** – swap the acquired tokens back into the denomination token.
- Run the workflow at a snapshot block height with optional prior transactions
  applied (to replay liquidity additions or tax-enabling steps).
- Capture structured results (`ProcessedTransaction`) for each leg so that
  downstream components can compute taxes, balance changes, and revert
  diagnostics.
- Provide rich failure context: revert reasons, decoded traces, and a fully
  populated `PoolBuySellSimulationResult`.

---

## Execution pipeline

The simulator lives in `entry.rs`. The steps below outline the control flow and
the corresponding helper modules:

| Step | Module | Responsibility |
| ---- | ------ | -------------- |
| 1 | `validation.rs` | Confirm that the pool address matches the on-chain factory for the token/denom pair (Uniswap V2/V3). |
| 2 | `buyer_setup.rs` | If the denom is WETH and we are not already dealing with Uniswap V4, deposit ETH into WETH inside the forked state. |
| 3 | `entry.rs` | Apply optional prior transactions (e.g. enable trading) and persist their processed form. |
| 4 | `entry.rs` | Approve the denomination token for the router (v2/v3) before attempting the buy. |
| 5 | `entry.rs` | Execute the **buy leg** via the correct router builder. |
| 6 | `entry.rs` | Execute the **approve leg** for the purchased token. |
| 7 | `entry.rs` | Optionally delay the sell by re-creating a new fork at a later block and replaying the previous legs. |
| 8 | `entry.rs` | Execute the **sell leg** using a builder that is compatible with fee-on-transfer tokens. |
| 9 | `balance_deltas.rs` | Extract denomination spent/received and token inflows/outflows from address balance changes. |
| 10 | `results.rs` | Assemble success booleans, tax computation, and diagnostic messages into the public result type. |

The simulator always uses `UnsignedTxChainSimulation::step_with_trace` so that
each leg records full call traces. `TxProcessor` processes the trace into a
`ProcessedTransaction`, feeding tax calculation and balance delta extraction.

---

## Builder choices and why they matter

### Denomination preparation
- **Builder:** `reth_chain_query::tx_builders::amm::uniswap_v4::build_weth_deposit_tx`
- **When:** Only when the denomination token is canonical WETH and the pool
  type is *not* Uniswap V4 (which has its own workflow).
- **Why:** Fee-on-transfer tokens often require exact balance accounting. Using
  WETH inside the forked state ensures we match the router’s expectation of ERC‑20
  semantics even for ETH denominated pairs.

### Buy leg
- **Current implementation gap:** `entry.rs` still calls `build_buy_swap(...)`,
  which is ETH/WETH-oriented and passes `test_amount` as the buy input. That is
  acceptable only for canonical ETH/WETH-denominated routes.
- **Target behavior for arbitrary denoms:**
  - **V2/Sushi:** `build_token_to_token_swap_v2`, input path `denom -> token`
  - **V3:** `build_token_to_token_swap_v3`
- **Rationale:** We buy using an ERC‑20 to ERC‑20 swap so that the amount of
  denomination tokens deducted from the buyer is observable in `ProcessedTransaction`.

### Token approve leg
- **Builder:** `reth_chain_query::tx_builders::build_approve_for_route`
- **Rationale:** Uses the router-specific spender and max allowance. The approve
  must happen *after* the buy so we can treat the resulting transaction as
  realistic (router allowance can be revoked in tax-heavy tokens).

### Sell leg
- **Critical choice:** Fee-on-transfer tokens (FLOKI, PEPE, etc.) will revert if
  we ask the router to compute exact `amountOut` before the transfer tax is
  applied. That is why we **must use builders that support fee-on-transfer
  semantics**.
  - **Current implementation gap:** `entry.rs` still calls `build_sell_swap(...)`.
    That is correct for ETH/WETH-style sells, but must be replaced or routed
    through a denom-aware token-to-token builder when `denom_address != WETH`.
  - **Uniswap V2 / SushiSwap:** `build_sell_swap_v2`, which encodes
    `swapExactTokensForETHSupportingFeeOnTransferTokens`.
  - **Token-to-token route:** When selling `token -> denom` where denom is
    not ETH, we use a fee-supporting builder that emits
    `swapExactTokensForTokensSupportingFeeOnTransferTokens`.
  - **Uniswap V3:** Always uses `exactInputSingle`; taxes must be handled by the
    pool itself.

Using `swapExactTokensForTokens` (the non-supporting version) is the fastest
way to trigger `TransferHelper: TRANSFER_FROM_FAILED`. The router calculates the
expected `amountOut` assuming no tax, but fee-on-transfer tokens mint a smaller
net amount. The router’s post-transfer balance check fails and the call aborts.

---

## Common failure modes and diagnostics

### `TransferHelper: TRANSFER_FROM_FAILED`
Most often caused by using the wrong sell builder. Confirm the sell transaction
calldata:
- `0x79…` (`swapExactTokensForETHSupportingFeeOnTransferTokens`) – OK for
  ETH/WETH denoms.
- `0x5c11d795` (`swapExactTokensForTokensSupportingFeeOnTransferTokens`) – OK
  for arbitrary denom tokens.
- `0x38ed1739` (`swapExactTokensForTokens`) – **Wrong for taxed tokens**, will
  revert if any tax is applied.

**FLOKI regression (Nov 2025).** A refactor swapped the sell leg to the plain
`swapExactTokensForTokens` builder. Fee-on-transfer tokens such as FLOKI deduct
tax during `transferFrom`, so the router’s post-transfer balance check failed
and the simulation returned `TransferHelper: TRANSFER_FROM_FAILED` even though
the on-chain sell succeeds. Restoring the supporting-fee builder
(`swapExactTokensForTokensSupportingFeeOnTransferTokens`) resolved the issue and
serves as a reminder to always use fee-tolerant routes for V2/Sushi pools.

### No denomination spend recorded
If the result shows `denom_spent = 0` for a WETH pair, check the balance delta
extraction logic. The balance calculator maps WETH transfers to the `"ETH"`
currency key and the extractor must use the same key when looking up balances.

**Fix applied.** `extract_token_balance_delta` now falls back from `"WETH"` to
`"ETH"` when reading `currency_net`, keeping denomination spend/receive fields
accurate for WETH pools.

### Approve fails for denomination token
Verify that the buyer account has the denomination token balance in the forked
state. For WETH this means confirming the initial deposit leg succeeded.

### Pool address validation error
Occurs when the provided pool does not match the factory result (caused by
token/denom ordering mistakes or stale pools).

---

## Historical Replay Failures To Fix

The old live-trading logs under `/home/nima/code/crypto/blockchains/eth/logs`
showed many cases where a transaction was already replayed by the real chain,
but our local viability simulation failed while reconstructing the same context.
Those failures should be treated as simulator/replay gaps unless the trace proves
the buy or sell genuinely reverted on-chain.

Log windows reviewed:
- `logs/live_trading/live_token_tracking_20251128_*.log`
- `logs/pools/PoolState_20251128_*.log`
- related logs through early December 2025

Observed failure buckets:

| Bucket | Count | What it means |
| ------ | ----- | ------------- |
| `PoolManager._load_and_register_v3_pool` / V2 failures | 294 | Pool discovery or metadata loading failed before the simulator could register the pool. |
| `Pool viability check failed` | 160 | The buy/approve/sell simulator failed while replaying prior transactions or executing the test route. |
| `token metadata simulation failed` | 250 | Metadata reads were attempted through simulation but the replay state or header was incomplete. |
| non-`None` `TradingStatus` errors | 1129 | The higher-level pool state recorded buy/sell/top-up failures after simulation. |

Important concrete errors:

- `State for block ... only exists in the live cache; use simulation APIs instead`
  - Seen mostly while loading V3 pools.
  - Cause: code asks for a direct historical `StateProvider` through
    `TxSimulator::get_chain_state_at_block`, but recent blocks may exist only in
    the live fork/cache path.
  - Fix direction: chain-query pool reads must use simulation/view APIs for live
    blocks instead of direct DB state.
  - Current live entrypoint: `LivePoolBuySellSimulator` selects
    `LiveTxSimulator::latest_state_status()`, so the default live check uses the
    live block processor's tracked state whenever it is ahead of MDBX.

- `Invalid block ... while reading UniswapV3 state`
  - Seen while loading V3 pool state.
  - Cause: V3 state lookup reads the header with `provider_factory.block_by_number`,
    which may return `None` for live/recent blocks even though the block context
    loader can still provide the header.
  - Fix direction: use `block_context_loader().load_block_header(...)` for the
    timestamp/header path, then keep state access on the same live-aware path.

- `transaction validation error: nonce too high`
  - Seen in prior tx replay, pool viability checks, and token metadata
    simulations.
  - Cause: selected prior transactions are replayed with their original nonces,
    but the selected list may skip an earlier transaction from the same sender.
    Reth correctly rejects nonce `N` when simulated account state still expects
    nonce `N - 1`.
  - Fix direction: either replay the full block prefix for every sender we touch,
    require contiguous per-sender prior tx lists, or add an explicit "selected
    prior replay" mode that normalizes the sender nonce before each selected tx
    and marks the result as approximate.

- `transaction validation error: gas price below basefee`
  - Seen in older viability and metadata simulations.
  - Cause: old replay paths preserved stale legacy gas prices below the target
    block base fee.
  - Fix direction: keep using the current fee policy that clamps unsigned/prior
    calls to the block header base fee; retest after execution sync.

- `UniswapV2Library: IDENTICAL_ADDRESSES`,
  `UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT`,
  `TransferHelper: TRANSFER_FROM_FAILED`, and buy-side
  `INSUFFICIENT_LIQUIDITY`
  - Seen in `TradingStatus` failures.
  - Cause: some entries are likely real token/pool behavior, but the biggest
    simulator-side suspect is denomination routing. The setup code understands
    `denom_address`, while the main buy/sell path still calls ETH/WETH-oriented
    builders.
  - Fix direction: make buy and sell fully denom-aware, then only treat remaining
    reverts as token behavior after confirming the constructed route and calldata.

Priority order:

1. Make the pool buy/sell path denom-aware for V2, Sushi, and V3.
2. Fix prior transaction replay semantics so successful on-chain setup txs do
   not fail locally because of skipped sender nonces.
3. Move V3 pool state/header reads onto live-aware simulation/view APIs.
4. Retest the old fee failures against the current base-fee clamping.
5. After the local Reth execution node is synced, turn representative log cases
   into regression examples under `examples/pool_analysis`.

---

## Development checklist

When extending or debugging the simulator:

1. **Pick the right builder** for the pool type and remember that fee-on-transfer
   tokens must always use “supporting” variants of sell swaps.
2. **Update both Rust and Python bindings** (`pyreth`) if you add fields to
   `PoolBuySellSimulationResult` or change configuration defaults.
3. **Keep balance extraction in sync** with the naming used by
   `AddressBalanceChangeCalculator`. Currency symbols (WETH→ETH) must match.
4. **Add regression examples** under `examples/pool_analysis` when you encounter
   a failure pattern. This keeps the token set up-to-date and protects against
   future refactors that regress fee handling.
5. **Leverage `enrich_failure_reason_with_trace`** when a revert appears. It
   simulates the failing leg in isolation and decodes the deepest revert so the
   CLI and notebooks show actionable information.

With these pieces aligned, the simulator provides repeatable answers about
trading viability at any historical block, which is crucial for the trading
analytics stack and automated tax classification.
