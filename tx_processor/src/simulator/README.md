Simulator — Action-Oriented Transaction Simulation

Purpose
- Provide a clean, deterministic way to simulate the on‑chain effect of actions (trades, approvals, sequences) at a specific historical block using only the local Reth DB.
- Take unsigned transactions (calldata only) from tx builders, execute them on a forked EVM state, and return rich, structured results (`ProcessedTransaction`) for downstream consumers (RL envs, analyzers, tests).

What is an “Action” here?
- A concrete transaction or sequence of transactions the agent can take:
  - Buy swap (ETH → token) on a specific route (e.g., Uniswap V2/V3/Sushi V2)
  - Sell swap (token → ETH) on a specific route
  - Approve (ERC‑20 `approve(spender, amount)`) to enable sells
  - Buy → Approve → Sell viability sequences (single pass)
  - [Planned] Single‑tx “permit + swap” flows (V3 `selfPermit + multicall`, Permit2/UniversalRouter)

How it fits together
1) Build unsigned tx(s) with reth_chain_query builders
   - Location: `reth_chain_query/src/tx_builders/*`
   - Stateless calldata constructors for V2/V3 buy/sell/approve and helpers like `spender_for_route`
2) Simulate at block b using tx_processor simulators
   - Single tx: `ProcessedTxProvider::process_transaction_from_unsigned_tx(unsigned, Some(b))`
   - Multi‑tx chain: `TxSimulator::start_simulation_chain(Some(b))` → `step_with_trace()` for each tx (e.g., approve → sell)
3) Get results as `ProcessedTransaction`
   - Includes decoded events, `address_balance_changes` (currency_net / token_net), `fees.gas_used`, status, logs
4) Consumers (e.g., RL envs) update their own state and compute reward from the deltas

Determinism & Fidelity
- Simulation is DB‑backed and block‑pinned: for a fixed (unsigned tx(s), block), results are deterministic.
- We never assume prior simulated effects made it into the DB. Each step re‑reads on‑chain state at its block and simulates exactly the tx(s) that would be sent now.
- Intra‑step sequences persist changes using a SimulationChain (e.g., approve → sell runs on the same fork at block b), but those changes are not written to the DB.

Typical flows
- Buy (ETH → token)
  - Build with `build_buy_swap(_with_min_out)` based on route and inputs
  - Simulate at block b
  - Extract tokens_out for the recipient from `ProcessedTransaction.address_balance_changes`

- Sell (token → ETH)
  - Read allowance at block b (outside, via provider) and decide whether to include `approve(MAX)`
  - Start SimulationChain at b; run approve (if needed) then sell
  - Extract ETH out, aggregate gas used, compute effective deltas

- Permit + swap (planned)
  - Build a single unsigned tx (e.g., Uniswap V3 `selfPermit + exactInputSingle` inside `multicall(bytes[])`)
  - Simulate exactly like any other unsigned tx; the permit signature (v,r,s) lives in calldata

End-to-end trading viability (Python → Rust → Python)
1. Python pool trackers (e.g., `py/eth_token/.../uniswap_v2_pool.py`) observe a control transaction and call `evaluate_trading_status`.
2. They build a `pyreth.PoolBuySellParameters`:
   - Inject the target token/pool, the paired denomination (USDC/WETH/…), the historical block (usually `tx.block_number - 1`), prior control transactions, and the token decimals.
   - Optionally attach a sealed header so the simulator runs against the exact parent block state.
3. The PyO3 bridge (`pyreth/src/simulator/pool_buy_sell_simulator.rs`) converts that struct into a Rust `PoolBuySellParameters`, enforcing that the denom address and token decimals are provided, and forwards it to `check_can_buy_sell_pool`.
4. `check_can_buy_sell_pool` (this crate) chooses the route, constructs the swap sequence, and executes it on a forked state:
   - Currently V2/V3 routes build `swapExactETHForTokens` for the buy leg (with a WETH→denom→token path when a denom token is specified), first replay any required prior transactions, then run `approve` and `swapExactTokensForETHSupportingFeeOnTransferTokens` for the sell.
   - Each step produces a `ProcessedTransaction`; failures return a `PoolBuySellSimulationResult` with `failure_reason`.
5. The result is surfaced back to Python as `PoolBuySellSimulationResult`, and the pool state updates `can_buy/can_sell`, taxes, and logging.

Limitations & TODOs
- The buy leg still assumes the agent funds the trade with ETH; reproducing on-chain `swapExactTokensForTokens` flows will require building/simulating the same calldata (USDC → token) and prefunding the buyer with the denomination balance.
- Taxes are inferred from the simulated round trip; if the token’s behaviour depends on caller balance or fee configuration that we do not populate (e.g., reflection tokens), results may diverge from live trades.
- Router gas parameters are fixed in config; callers should tune them when replaying high base-fee blocks to avoid `GasPriceLessThanBasefee`.

Data extraction (ProcessedTransaction)
- `address_balance_changes` by address:
  - `currency_net["USDC"|"USDT"|"DAI"|"ETH"]` for known tokens/ETH
  - `token_net[checksum(token_addr)]` for arbitrary ERC‑20s
- Gas: `fees.gas_used`; higher layers compute cost via base fee + tip

Where to look
- Single‑shot orchestration: `src/processed_tx_provider/provider.rs`
- Core simulation chain: `src/unsigned_tx_chain_simulator.rs` (via `TxSimulator::start_simulation_chain(at_block)`) 
- Viability analyzer (buy→approve→sell): `src/simulator/{buy_swap_simulator.rs, sell_swap_simulator.rs, pool_buy_sell_simulator.rs, cross_venue_buy_approve_sell.rs}`

Observations from the 2025-10-28 Live Run
- Source: `logs/pools/PoolState_20251028_203459.log` (captured before the pool validation fixes).
- Dominant failure (≈80 % — 1 676 hits): `Buy transaction failed (revert: Contract 0x7a250… reverted without returning data)` on Uniswap V2 paths. These were router fallbacks caused by computing a pool address while the local Reth state had not yet indexed the pair (or the pair had already been drained). With the post‑fix factory check, the same cases now surface as `No Uniswap V2 pool found…` instead of reaching the router.
- Secondary buy failures:
  * 43× `UniswapV2: TRANSFER_FAILED` — fee-on-transfer / blacklisted tokens (e.g., 0x3D72A7dA… at block 23669052) that reject the router as sender.
  * 38× `UniswapV2Library: INSUFFICIENT_LIQUIDITY` — pools detected but reserves were zeroed immediately after a rug (`Denom_removal` alerts corroborate these blocks).
  * 14× `Contract 0xE592… reverted without returning data` — identical symptom on Uniswap V3 when `getPool` returned the zero address. The new guard in `check_can_buy_sell_pool` now blocks these at the factory lookup as well.
- Sell leg regressions:
  * 56× `Simulation failed at step SELL (TransferHelper: TRANSFER_FROM_FAILED)` — buy succeeded, but tokens enforce post-buy lockups (example: 0x1dfdB02d… at block 23668386). These should remain flagged as non-tradeable even after the refactor.
  * 3× `Simulation failed at step SELL (UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT)` — taxed sells where the post-buy balance was entirely burned.
- Prior-transaction replays:
  * 114× `TransferHelper: TRANSFER_FROM_FAILED`, 35× `ERC20: transfer amount exceeds allowance`, plus various custom errors when we attempted to replay a user’s “enable trading” or approval tx without mirroring their original allowances. Actions pulled into `PoolBuySellParameters.prior_txs` must either include all prerequisite approvals or be skipped (otherwise the simulator starts from a clean account and the replay fails deterministically).
- Live-state caveat: even though we now fetch sealed headers from the RPC node for the block we are replaying, the local Reth datadir must be caught up to the same height. When the DB was still indexing 236722xx, brand-new pools produced the router reverts above. If we observe the same pattern again:
  1. Call `simulate_view_function(UNISWAP_V2_FACTORY, getPair(token, denom), block)` to verify the pool address exists at the target block before building swap calldata.
  2. Query `getReserves()` on the resolved pool to rule out rugs (zero liquidity) versus missing state.
  3. For sell reverts, inspect `erc20.allowance(buyer, router)` at the replayed block to confirm whether an enable transaction is required.
- Historical regression: FL0KI (token `0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E`) previously hit the 0x7a250… router revert in this log. After enforcing denom addresses and validating pairs, the current simulator reports the pool as missing instead of queuing a failing swap, which is the expected behaviour.

Design split (by crate)
- reth_chain_query: tx builders (unsigned txs), provider (read‑only chain data)
- tx_processor (this crate): simulators + `ProcessedTransaction`
- External strategy/training envs: map actions → unsigned tx(s), simulate, apply deltas, and compute rewards
