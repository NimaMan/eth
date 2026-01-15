# JaredFromSubway Profit Study

## Objective
The goal of this notebook folder is to build a reproducible analysis that answers one question: **How has the searcher behind `0xae2Fc483527B8EF99EB5D9B44875F005ba1FaE13` (aka “jaredfromsubway”) generated profit over the last month?**  
We want to quantify every strategy the address uses (sandwiches, cross-venue arbitrage, direct liquidations, bribe income, etc.), measure realised profit/loss in both ETH and USD terms, and produce supporting evidence for each trade family.

## Data Sources & Tooling
The repository already exposes everything we need without leaving our own infrastructure:

1. **Reth processed index** – use it to enumerate every transaction hash that involves the target address or its helper contracts (e.g. `0x1f2F10…DF387`). The index stores balance deltas, decoded events, and internal transfers.
2. **`PyReth` Python bindings** – provide convenient accessors (`ProcessedTxProvider`, `ChainQuery`) to pull decoded data directly from the index or query on-chain state at specific blocks.
3. **Local RPC node** – for any missing context we can issue JSON-RPC calls against our own node (no reliance on third-party endpoints).
4. **Scripts in this folder** – `sandwich_breakdown.py` provides per-bundle inspection, and `latest_tx_profit.py` now walks the newest processed transactions to aggregate net flows across Jared’s key addresses.

## Flow Network Model

To understand where extracted value ultimately lands we model every processed transaction touching the bot as a directed weighted graph:

- **Nodes** are Ethereum addresses. We annotate the ones we know (`jared_eoa`, `bundle_contract`, payout wallets, frequently interacting contracts) and keep the rest as “unknown” until classified.
- **Edges** represent net ETH-like value flowing from one address to another inside a transaction. For each processed transaction we:
  - Convert both `currency_net` (native ETH) and any WETH movements (`token_net` entries for WETH) into a single ETH-denominated delta per address.
  - Add the gas fee as an edge from the transaction sender to the coinbase/builder (the fee recipient implicit in the balance changes).
  - Split inflows/outflows proportionally when multiple addresses participate, so the sum of all incoming edge weights equals the sum of outgoing weights for that transaction.
- **Weights** are the ETH amounts (positive for inflow to the destination node, negative for outflow from the source).

By aggregating edges across a window (latest N transactions or a time range) we derive:

1. **Net node balances** – total ETH extracted or spent per address.
2. **Top counterparties** – who funds ultimately, consistently flow to or from.
3. **Subgraph structure** – clusters of addresses representing payout hubs, liquidity venues, or cross-chain exits.

The network builder will live alongside the profit scripts and expose two primary entry points:

- `NetworkBuilder.build(transactions)` → returns the graph object (nodes, edges, metadata).
- `NetworkReporter.print_summary(graph, top_n)` → prints top inflow/outflow addresses and optionally renders the graph in Graphviz/JSON for downstream analysis.

This graph view complements the per-transaction P&L metrics and helps identify new wallets worth adding to the watch list.

## Analysis Plan
We will follow a three-stage pipeline:

1. **Discovery / Grouping**
   - Use the processed transaction provider to fetch every transaction touching the EOA or known contracts over the last 30 days.
   - Cluster transactions into bundles by `(block_number, from_address)` and track helper addresses (e.g. payout EOAs that receive WETH/ETH from the bot).
   - Tag each bundle by strategy using on-chain features: Uniswap V2 sandwich pattern (front + victim + back), Kyber/Uniswap V3 hop arbitrage, flash-loan liquidation, etc.

2. **Accounting**
   - For each bundle, compute the net change in ETH/WETH for the bot, including transfers to payout wallets and internal balance changes.
   - Convert profit/loss to USD using block-level oracles (either from our `eth_prices` crate or Chainlink feeds archived in the index).
   - Record gas spend, bribe tips, and any ERC-20 inventory changes so totals are net of costs.

3. **Reporting**
   - Produce per-strategy summaries (counts, total profit, average ROI).
   - Highlight outliers (e.g. exceptionally profitable or failed sandwiches).
   - Document qualitative observations (preferred pools, typical slip demanded, time-of-day patterns).

Automation scripts will live alongside this README; each should be runnable via `python <script>.py --start <block> --end <block>` so we can regenerate numbers as the index grows.

## Immediate Next Steps
1. Build a “bundle ledger” script that streams processed transactions for the target addresses, groups them, and dumps profit metrics into CSV/Parquet.
2. Extend `sandwich_breakdown.py` (or a sibling) to classify bundles automatically.
3. Cross-check totals by comparing month-start vs month-end ETH balances to ensure accounting matches observed wallet delta.

Once these steps are complete we will be able to write the final assessment detailing precisely how the bot made money during the period.***
