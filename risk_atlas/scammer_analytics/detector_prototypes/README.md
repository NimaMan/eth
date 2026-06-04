# Detector Prototypes

## Objective

Actor-level **detector notes** — candidate signals for catching a malactor early,
captured here before promotion into Rust source modules. This feeds pillar 1
(track & flag the malactors) and the "detection coverage" stat (`../DESIGN.md`
§3: did our LIVE detector emit a holder-balance-drain / confiscation risk event
for a case?).

## Contents

Prototype detector notes (text/markdown), each describing a signal, the on-chain
evidence it keys off, and its promotion status. Initial candidates:

- repeated creator lifecycle (deploy → fund → open → remove, same operator);
- same bytecode / name / symbol token staging (`[[staged_tokens]]` family);
- LP approval followed by direct creator liquidity removal;
- CREATE/selfdestruct forwarder fanout (the `ForwarderTrace` pattern);
- repeated remover or final-sink reuse;
- holder-balance / vault drain with no normal `Transfer` log (the §3 "Ask-1"
  detector — see the `eth_0x02467dd0_session_vault_balance_drain` case fixture).

## How it's produced / how it connects

Notes here are hand-authored from case evidence (`../cases/`), then promoted into
detector code once stable. The forwarder-fanout signal is already realized in
`../../src/scammer_analytics/analyzer.rs` (`detect_forwarder_trace`). Detection
coverage is reported per case and rolled up per scammer (`../scammers/`) /
landscape via the read-model (`eth_chain_server/src/read_models/analytics/scammer.rs`).

See `../DESIGN.md` and the parent `../README.md`.
