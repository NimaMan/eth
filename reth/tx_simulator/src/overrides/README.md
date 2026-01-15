overrides — State and Block Overrides (Planned)

Goal
- Accept RPC-style state and block overrides, apply them to the EVM environment for a single call/tx or a sequence, mirroring Reth’s debug infrastructure.

Background
- Reth applies overrides via EvmOverrides (state + block) when handling debug_trace* calls with overrides.

Planned types
- struct StateOverrideAccount { balance: Option<U256>, nonce: Option<u64>, code: Option<Bytes>, storage: Option<Vec<(B256,B256)>> }
- type StateOverrides = HashMap<Address, StateOverrideAccount>
- struct BlockOverrides { number: Option<U256>, timestamp: Option<U256>, basefee: Option<U256>, gas_limit: Option<U256>, coinbase: Option<Address> }

Planned APIs
- simulate_unsigned_with_overrides_at_block(tx, block, tracer, state_overrides: Option<StateOverrides>, block_overrides: Option<BlockOverrides>) -> eyre::Result<TraceOutput>
- trace_block_with_overrides(block, tracer, state_overrides, block_overrides) -> eyre::Result<Vec<TraceOutput>>

Reth references
- EVM overrides preparation: rust/reth/crates/rpc/rpc/src/debug.rs:558-573, 540-600

Notes
- Overrides are applied once before the first tx in a sequence. For bundles, we will reuse Reth’s pattern (apply then commit between txs).

