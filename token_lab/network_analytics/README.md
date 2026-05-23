# Network Analytics

Token and pool graph case studies live here when the goal is to understand
first-order token relationships plus second-order ETH/denom fund-flow context.

Use this folder for work that is broader than one parity bug:

- token transfer and pool interaction graph shape;
- high-signal addresses, seed selection, and clusters;
- background fund-flow edges that explain coordinated wallets;
- protocol/router labels that should be filtered from risk scoring;
- detector ideas that should later move into `eth_token`, `eth_chain_server`,
  `mempool_processor`, or `alpha`.

## Folder Shape

Use a stable case folder name:

```text
<token_symbol_or_name>_<network_context>_<start_block>
```

Each case should contain:

- `README.md`: narrative and current interpretation;
- `analysis.toml`: machine-readable scope, addresses, blocks, and key txs;
- `artifacts/`: generated JSON, command output, reserve samples, and graph data.

Generated artifacts belong under the case `artifacts/` folder. Keep committed
artifacts small and human-auditable unless the file is needed as a regression
fixture.

## Current Cases

| Case | Token | Scope | Status |
| --- | --- | --- | --- |
| `vyp_network_liquidity_drain_25077324` | `0x8ceda8619ad186e7c9bca77734e48c8f518f5d01` | V2 pool drain plus second-order fund-flow clusters | investigating |
