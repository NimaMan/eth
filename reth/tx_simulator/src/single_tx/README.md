single_tx — Single Transaction Simulation

Purpose
- Simulate a single transaction against a chosen block environment using the local Reth DB (no RPC).

Modules
- unsigned.rs: Build a TxEnv from UnsignedTransaction and execute with optional tracing.
- signed.rs: Recover signer for TransactionSigned and execute with optional tracing.

Notes
- Envs are derived from canonical headers via Reth providers for equivalence with Reth debug RPC.
- Traces use geth-compatible TracingInspector configs and export CallFrame structures.

