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

## Active Chain-Parity Items

| Status | Item | Core Question |
| --- | --- | --- |
| confirmed | [Compass V2 vault helper-route chain parity](../../investigations/compass_v2_vault_helper_route_chain_parity_25122982/) | The chain has same-block helper-route sells, but Alpha's classic V2 route and deployed V2 vault route are not yet proven execution-equivalent at the historical block plus transaction index coordinate. |
