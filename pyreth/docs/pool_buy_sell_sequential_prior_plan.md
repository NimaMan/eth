# Pool Buy/Sell Simulator: Sequential Prior Transaction Support

## Objective
Enable the simulator to replay an ordered sequence of prior transactions (rather than a single tx) before attempting the buy/approve/sell probe. This allows us to correctly simulate launches that rely on contract helpers within the same block.

## Motivating Example
- **Token:** Mind of Pepe (`0x2A179Bfab23c71c733864776bEa6e0daB9eCD79e`)
- **Block:** `23606650`
- **Key transactions (sorted by nonce for deployer `0x539714c8aA548f98e6D564486BFb6658326840A1`):
  1. `0x027399b9a14e8e7c1d86b8f82f205abdef3b5c4ccbc2475cba70dcbbcf187b2b` – contract creation (nonce 0)
  2. `0x7468044e32eeab38ed8afe6ee17648a8effa6057da9f152be03ff3a2aea7af22` – `openTrading()` (nonce 1)
  3. Additional snipers/aggregator helpers (e.g. `0x2b7444bedda25c00470797ae772c2c48b461c3feb198cf6d95243471b2f0456b`, `0x73fe4231da417596084c9aedbbd70409100ff0c5491214d0e5abb43cde0b059f`, …).

Replaying only TX #3 without TX #1/#2 leaves the sandbox with no token/pair, causing immediate reverts.

## Implementation Plan
### 1. Example Guidance
- Add a new example under `pyreth/examples/pool_buy_sell_simulator/`:
  - Download block `23606650`, extract the deployer sequence (above).
  - Convert each transaction to `ProcessedTransaction` via existing helpers.
  - Feed the sequence into the simulator before running the standard buy/approve/sell block.
  - Document expected outcomes (buy/sell should succeed once the full sequence is applied).

### 2. API Changes
- Update `PoolBuySellParameters` (`../tx_processor/src/simulator/types.rs`):
  - Replace `prior_tx: Option<ProcessedTransaction>` with `prior_txs: Vec<ProcessedTransaction>`.
  - Maintain backward compatibility in Python binding by accepting both single and list parameters.
- Adjust result struct to optionally return the list of `prior_transactions` for debug/logging.

### 3. Simulator Logic
- In `../tx_processor/src/simulator/pool_buy_sell_simulator.rs`:
  - Iterate through `prior_txs` in order; for each, set the `UnsignedTransaction` and run `step_with_trace` on the simulation chain.
  - Collect individual `ProcessedTransaction` results for diagnostics.
  - Only after the entire sequence succeeds, run our standard buy/approve/sell.
  - If any prior fails, surface its error directly (include hash/nonce in failure reason).

### 4. Python Bridge
- Update `pyreth/src/tx_processor/py_processed_transaction.rs` + `processed_tx_bridge.rs` to allow passing a list of prior tx dicts.
- Expose the vector in the `PoolBuySellSimulationResult` returned to Python (for logging/debugging).

### 5. Callers & Tests
- Adjust all Rust examples/tests to the new API (including the basic pool checks, tax demos, etc.).
- Update mempool processor `SimulationManager`: maintain a short in-memory list of pending creator txs per deployer/token; supply that ordered list when building `PoolBuySellParameters`.
- Extend Python tests/examples to cover sequential prior replays.

### 6. Validation
- Use the new example to confirm the simulator reproduces Mind of Pepe block correctly.
- Run existing unit/integration tests for tx processor, mempool processor, and Python bindings.
- Add regression tests ensuring the simulator handles empty prior lists (legacy behavior) and multiple entries.

### 7. Documentation
- Update simulator README (`blockchains/eth/mempool_processor/src/simulator/README.md`) with instructions on collecting sequential prior transactions and how the new API is used.
- Mention the specific MIND block example as a “known-good” scenario.

## Notes
- The transaction ordering should be by `(block_number, transaction_index)` or `(nonce)` depending on context (nonce for identical sender; index otherwise).
- Consider a bounded look-back window (same block + maybe earlier pending transactions) to avoid unbounded growth.
- Ensure gas limits and fee overrides from each prior tx are respected.

## Developer Guidelines

### 1. Example Expectations
- The example script should:
  - Fetch block **23606650** and parse the full sequence of tx hashes listed above.
  - Convert each transaction to a `ProcessedTransaction` via the existing processed-tx provider (or Python bridge) *in the exact nonce order*.
  - Supply the vector to the simulator and print:
    - Confirmation that each prior tx applied successfully (hash + gas limit/fee info).
    - The final buy/approve/sell outcome (`can_buy`, `can_sell`, tax percentages, tokens received, ETH spent).
- Expected result: buy + sell should succeed with sensible tax numbers (close to 0% on deploy), matching real block behavior. Any failure should surface the failing prior tx hash.

### 2. Implementation Steps (Detailed)
1. **Data types**
   - Modify `PoolBuySellParameters` to expose `prior_txs: Vec<ProcessedTransaction>`.
   - Update result structs to carry `prior_transactions: Vec<ProcessedTransaction>` for inspection.

2. **Simulator core**
   - Iterate through `prior_txs`; for each:
     - Build an `UnsignedTransaction` from the processed payload.
     - Apply fee policy only when the payload lacks explicit fee data.
     - Call `step_with_trace`, collect the `ProcessedTransaction`, and abort on the first revert.
   - After all priors succeed, proceed to the current buy/approve/sell block.

3. **Python bridge / FFI**
   - Allow `prior_txs` to be passed as a list of dicts in Python (`processed_tx_bridge.rs`).
   - For backward compatibility, accept a single dict and wrap it into a one-element vec.

4. **Callers**
   - **Historical pipeline (`tx_processor`)**: when replaying a block, gather all earlier transactions for the same deployer/token before the current tx; pass the vec to the simulator.
   - **Mempool processor**: maintain an in-memory per-deployer queue of pending txs (bounded) and supply it when running the buy/sell probe.
   - Update examples/tests to supply the new field.

5. **Testing**
   - Unit tests: verify that an empty vec behaves as before, and that a vec with >1 entries executes them in order.
   - Integration tests: run the Mind-of-Pepe example plus at least one synthetic sequence that intentionally reverts mid-way (to confirm errors propagate).
   - Python tests: ensure `prior_transactions` round-trip correctly and the dict interface is stable.

### 3. Expected Output for Example Script
```
Applying prior transactions for Mind of Pepe (block 23606650)
  ✔ 0x0273…b2b (nonce 0) — contract deployment applied
  ✔ 0x7468…af22 (nonce 1) — openTrading + addLiquidity applied
Replaying helper tx 0x2b74…56b:
  ✅ Buy: can_buy=true, tokens_received=XXX, denom_spent=YYY
  ✅ Sell: can_sell=true, denom_received=ZZZ
  Taxes: buy=0.0%, sell=0.0%
```
- Any deviation should be logged explicitly (e.g., which prior failed and why).

### 4. Follow-up Checklist
- [ ] Commit the example and README updates.
- [ ] Ensure CI lint/tests cover the new API.
- [ ] Coordinate the mempool processor changes so both repos stay in sync.
