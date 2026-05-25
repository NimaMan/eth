# Parity Tools

Parity tools replay historical behavior and compare our simulator against chain
truth.

Minimum checks for a transaction replay:

- original tx succeeds or reverts the same way;
- emitted log topics match expected critical logs;
- pool reserves match after replay;
- token and denom `balanceOf(pool)` match after replay;
- buy and sell outcome matches the observed chain behavior when applicable.

When parity fails, the case must classify the cause:

- missing prior transaction setup;
- incorrect token metadata or decimals;
- incorrect pool orientation;
- missing control transaction replay;
- simulator EVM/state bug;
- expected failure because chain behavior depends on private or unavailable state.

## Targeted Transaction Replay

`ProcessedTxProvider::process_transaction_by_hash` is the local path for
block-prefix replay of an observed transaction by hash. It must request
`callTracer` options from the fused replay engine, because that engine is the
fast path for call-trace-compatible options and the provider expects a
`CallTracer` result.

This matters for SSS block `25,041,123`: targeted replay now succeeds for the
creator pool-token transfer at tx `197`, creator `sync()` at tx `198`, and WETH
drain sell at tx `200`. That proves transaction-level processing can reproduce
the coordinates before we promote any live signal.

## Historical Route Replay

When the production contract did not exist at the historical target block, use
a code overlay only to answer route capability. Record that distinction
explicitly:

- chain truth: what actually happened at the historical block and transaction
  index;
- route capability: whether Alpha's current executable route would succeed from
  that historical prestate if it held the asset.

For SSS block `25,041,123`, the deployed V2 vault has no code at the historical
coordinate. Overlaying the current vault code plus a synthetic `1 SSS` vault
balance proves the route still fails before tx `197` with
`TransferHelper: TRANSFER_FROM_FAILED`. That makes the SSS control-transfer
pattern avoid-only for this route unless a future route-specific replay proves
otherwise.

## Active Chain-Parity Items

| Status | Item | Core Question |
| --- | --- | --- |
| confirmed | [Compass V2 vault helper-route chain parity](../cases/compass_v2_vault_helper_route_chain_parity_25122982/) | The chain has same-block helper-route sells, but Alpha's classic V2 route and deployed V2 vault route are not yet proven execution-equivalent at the historical block plus transaction index coordinate. |
| confirmed | [SSS creator transferFrom pair drain](../cases/sss_creator_transfer_from_pair_drain_25041123/) | Chain/token-builder range parity is reproduced, exact chain truth shows tx `197` creator `transferFrom(pair, ...)`, tx `198` creator `sync()`, and tx `200` WETH drain, and Alpha's current deployed V2 vault route fails even before tx `197`; promote as avoid-only detector research, not exit recovery evidence. |
