# Soleth Workspace (Solidity)

Soleth is the on-chain execution layer for the Baygus stack. It hosts the Solidity contracts that the Rust (`reth`) and Python (`pyeth`) tooling target when they decide to buy, sell, unwind, or route liquidity. Think of Soleth as the set of on-chain primitives our agents call after off-chain analysis/simulation is done.

## Relation to the rest of the stack
- **reth/** (Rust): simulators, calldata builders, transaction processors, mempool signal detectors. These produce decisions (buy/sell/skip), gas estimates, and calldata.
- **eth_tx_executor** (Rust, under `reth/eth_tx_executor`): execution harness that consumes trades/alerts, builds/sends signed transactions to mainnet. Targets Baygus Router when the agent chooses to route via our contracts.
- **pyeth/** (Python): bindings and higher-level orchestration for agents/services that consume the Rust outputs.
- **soleth/** (Solidity): the contracts those agents actually execute against on-chain (Baygus Router and future adapters).

Data/control flow (typical cycle):
1) Mempool or historical signal → `tx_processor`/`mempool_processor` in Rust simulates viability.
2) Agent decides to trade → `reth_chain_query` builds router calldata (V2/V3/V4 paths) and simulates via `tx_simulator`.
3) `eth_tx_executor` (or an equivalent sender) signs and broadcasts the transaction to the on-chain Baygus Router (Soleth). The router executes swaps/settlement on-chain.
4) Post-trade, receipts and traces flow back into Rust/Python for accounting and future decisions.

## Projects here
- **baygus-router/**: primary contract project. A command-driven router that can execute single-hop and multi-hop swaps across Uniswap v2/v3/v4 (and adapters for Balancer/Curve in progress). Foundry-based, with tests and docs under this folder.
- (Space for future Solidity components will mirror this structure.)

## Responsibilities of Soleth (on-chain executor)
- Provide the on-chain router entrypoints the off-chain agent targets.
- Enforce safety (reentrancy guards, bounded slippage, calldata sanity) and execute swaps/settlement.
- Support bounded builder/validator payment commands for private-bundle execution when the off-chain
  executor chooses that path.
- Optimize gas paths (e.g., permit/infinite approvals, flash accounting for multi-hop, calldata-light encodings).
- Serve as the canonical contract that Rust/Python builders reference for ABI/bytecode when constructing transactions.

## Walk-through Scenarios
- **Buy then sell via Baygus Router (V2/V3):** Off-chain agent (Rust) builds a `buyThenSell` command sequence, simulates it, then submits a signed tx to the router. Router pulls tokens (permit or approve), routes swaps, returns proceeds. Gas advantage comes from single call vs multiple user txs.
- **Single-leg buy (first-time buyer):** Agent uses permit to avoid a separate approval; router warms storage minimally and executes one swap. Good for cold-start wallets or rapid entries.
- **Multi-hop V3 path:** Agent encodes WETH→USDC→DAI; router should flash-account intermediate hops (settle only at the end) to minimize external transfers and gas.
- **Unwind/exit:** Agent sends a sell command to offload position; router executes the sell path and returns base asset, supporting deadlines/slippage guards.
- **Cross-protocol chaining (future):** Router adapters sequence V2/V3/Balancer/Curve legs without multiple user txs, reducing calldata and redundant approvals.

## State of Baygus Router (quick read)
- Foundry project: `baygus-router/` with Solidity sources under `contracts/` and tests under `contracts/test/`.
- Current gaps (from recent audit and gas review):
  - Gas: V2 single-hop buy+sell is ~92k gas heavier than Uniswap V2 reference due to command dispatch/extra calls; needs a fast path and approval/permit polish.
  - Multi-hop: needs flash accounting so intermediates don’t settle every hop.
  - Approvals: replace repeated approvals with permit/one-time max approvals; use custom errors everywhere.

## What to build/improve next
- Add fast-paths for common V2/V3 single-hop routes to close the gas gap vs native routers.
- Implement flash accounting for V3/v4 multi-hop to avoid per-hop settlement.
- Standardize permit/infinite-approval handling and custom errors; adopt transient storage for guards/buffers where chain supports Cancun.
- Keep ABI/bytecode artifacts in sync for Rust/Python builders; document any selector/encoding changes.
- Near-term focus: V2 pools first (single-hop fast path + approval/permit polish) so `eth_tx_executor` can confidently route through Baygus with gas parity or better versus Uniswap V2.
