# scammer_analytics (Rust module)

## Objective

The actor-centric Rust module behind the `eth_risk_atlas` **`scammer-case`** CLI
subcommand. Given a `case.toml`, it replays the case's key transactions against
the local reth datadir and writes the per-case fund-flow artifacts and an
evidence packet — step 1 of `../../scammer_analytics/DESIGN.md` §10 (per-case
artifacts). Where `scam_analytics` is token/pool-centric, this follows the
**actor**: creator, funder, remover, forwarder, final sink, and staged token
campaigns. This is the producer the whole workspace rolls up from (pillars 1 & 3).

## Contents (modules)

- `mod.rs` — module root; re-exports `ScammerCaseAnalyzer` and the model types.
- `model.rs` — data contracts:
  - `ScammerCaseConfig` (parsed from `case.toml`; `serde(deny_unknown_fields)`),
    `KeyTransactionConfig`, `StagedTokenConfig` — `ordered_transactions()`
    de-dupes and orders key txs, staged-token txs, then `forward_txs`.
  - `ScammerCaseReport` { `config`, `summary`, `transactions` },
    `ScammerCaseSummary` (tx counts, creator direct ETH in/out, total forwarded
    ETH, unique forwarder sinks, unique token contracts touched).
  - `TransactionEvidence`, `EthMovementEvidence`, `TokenMovementEvidence`,
    `ForwarderTrace`.
- `analyzer.rs` — `ScammerCaseAnalyzer`:
  - `analyze_case_file()` / `analyze_config()` — loads each tx via
    `tx_processor::ProcessedTxProvider`, extracts directed ETH/token movements
    via `tx_fund_flow`, sorts by (block, tx_index), summarizes vs the suspect.
  - `detect_forwarder_trace()` — recognizes the CREATE/CREATE2 + selfdestruct
    forwarding pattern (and an internal-forward fallback) into a final sink.
  - helpers: `parse_address`/`parse_hash` (checksummed-address aware),
    `wei_to_eth`, address/hash formatting.
- `writer.rs` — `ScammerCaseReport::write_artifacts(case_dir)` writes
  `artifacts/tx_fund_flow/transactions.json`, `artifacts/traces/forwarder_traces.json`,
  `artifacts/tx_fund_flow/eth_edges.csv`, and `reports/evidence_packet.md`.

## How it's produced / how it connects

Invoked from `../main.rs` (the `scammer-case` arm):

```bash
cargo run -p eth_risk_atlas -- scammer-case \
  risk_atlas/scammer_analytics/cases/<case_id>/case.toml [reth_datadir]
```

(default reth datadir from `$RETH_DATADIR`). It writes into the matching
`scammer_analytics/cases/<case_id>/` folder. The richer buyer-outcome and
cash-out-trace artifacts come from `eth_token` examples (see
`../../scammer_analytics/pipeline/README.md`). Downstream, the read-model
`eth_chain_server/src/read_models/analytics/scammer.rs` consumes these case
artifacts to build the scammer / exchange / landscape rollups.

See `../../scammer_analytics/DESIGN.md` and `../../scammer_analytics/README.md`.
