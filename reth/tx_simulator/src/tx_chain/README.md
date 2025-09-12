tx_chain — Stateful Sequential Simulation

Purpose
- Execute multiple transactions in order with state persistence between steps (e.g., buy → approve → sell, MEV bundles).

Modules
- unsigned.rs: Interactive chain for UnsignedTransaction with inspector fusing.
- signed.rs: Interactive chain for TransactionSigned with inspector fusing.
- bundle.rs: Batch execution API (provide the whole sequence upfront).

Notes
- Forked state overlays writes; canonical state is read from Reth DB.
- Inspector is reused and fused between txs to match Reth’s block/bundle tracing behavior.

