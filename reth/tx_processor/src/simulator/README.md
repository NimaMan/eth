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
   - Location: `rust/reth_chain_query/src/tx_builders/*`
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

Data extraction (ProcessedTransaction)
- `address_balance_changes` by address:
  - `currency_net["USDC"|"USDT"|"DAI"|"ETH"]` for known tokens/ETH
  - `token_net[checksum(token_addr)]` for arbitrary ERC‑20s
- Gas: `fees.gas_used`; higher layers compute cost via base fee + tip

Where to look
- Single‑shot orchestration: `src/processed_tx_provider/provider.rs`
- Core simulation chain: `src/unsigned_tx_chain_simulator.rs` (via `TxSimulator::start_simulation_chain()`)
- Viability analyzer (buy→approve→sell): `src/simulator/erc20_token_buy_approve_sell_tx_simulator/*`

Design split (by crate)
- reth_chain_query: tx builders (unsigned txs), provider (read‑only chain data)
- tx_processor (this crate): simulators + `ProcessedTransaction`
- eth_price_leverage: RL envs that map actions → unsigned tx(s), simulate, apply deltas, and compute rewards

