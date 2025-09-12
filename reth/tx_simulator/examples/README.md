# Transaction Simulator Examples (Index)

This directory showcases how to use `tx_simulator` across common scenarios. This index is intentionally high-level; for API specifics see the "Detailed Design and API Spec" section in `../README.md`.

## Basic
- basic/verify_database_setup.rs: Verify Reth DB access and latest block
- basic/unsigned_transaction_example.rs: First unsigned call simulation
- basic/contract_method_simulation.rs: Read-only contract state queries
- basic/contract_method_simulation_weth_total_supply.rs: Simple totalSupply() read example
- basic/trace_extraction_example.rs: Extract geth-compatible call traces

## Sequential (Stateful)
- sequential/sequential_eth_transfers_with_state_persistence.rs: ETH transfers with persisted state
- sequential/auto_nonce_management_example.rs: Automatic nonce detection and increment
- sequential/mev_sandwich_bundle_example.rs: Frontrun/victim/backrun demo as a bundle

Token workflows (unsigned chains):
- sequential/specific_tokens/buy_approve_then_sell_floki.rs
- sequential/specific_tokens/buy_approve_then_sell_pepe.rs
- sequential/specific_tokens/buy_approve_then_sell_usdc.rs

Signed chain:
- sequential/buy_approve_sell_signed_chain_uniswap_v2.rs

## Performance
- performance/rpc_vs_direct_simulation_benchmark.rs: RPC vs direct DB comparison
- performance/inspector_fusing_test.rs: Inspector reuse benchmark

## Advanced
- advanced/timeout_handling_example.rs: Per-tx timeout patterns
- advanced/revert_reason_decoder_example.rs: Human-readable revert decoding

## Block Tracing
- block/trace_block_transactions.rs: Trace all txs in a block
- block/verify_block_trace_rpc_equivalence.rs: Compare local traces with RPC

## Running
- Ensure a synced Reth DB is available (default: `~/.local/share/reth/mainnet`).
- Run any example: `cargo run --example <name>`
- Use `--release` for better performance on heavy workloads.

## Notes
- Examples that alter state simulate against a forked overlay; no writes hit disk.
- Some examples assume funded mainnet accounts for realism; view examples do not.
