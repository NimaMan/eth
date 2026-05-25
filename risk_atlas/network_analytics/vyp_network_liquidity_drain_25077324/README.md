# vyp Network Liquidity Drain

This case studies token `vyp` from the token network outward: first-order token
and pool relationships, then second-order ETH/WETH fund-flow context around the
addresses selected from that network.

## Scope

- token: `0x8ceda8619ad186e7c9bca77734e48c8f518f5d01`
- symbol: `vyp`
- pool: `0xd8e654a9b4e861b54adb903ada20e3787dbd0079`
- denom: `WETH`
- pair type: `Uniswap V2`
- activity window: blocks `25077324..25081722`
- expanded replay window: blocks `25077304..25081742`

## Current Chain Facts

The pool is the canonical V2 pair for this study:

```text
token0 = 0x8ceda8619ad186e7c9bca77734e48c8f518f5d01
token1 = 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

Reserve samples show a severe WETH drain:

| Block | Token reserve | WETH reserve |
| ---: | ---: | ---: |
| `25077324` | `803265000.000000010` | `1.245731097010885559` |
| `25077493` | `36976006.197668655` | `28.284524657374708461` |
| `25077595` | `588081647.519733296` | `0.000018052161427478` |
| latest checked | `378848179.182689755` | `0.000028052161427478` |

The WETH reserve fell by about `99.9999%` from the observed peak.

## Key Transactions

- token creation:
  `0x4f55f16ce6d60a16155d2cc1264d16e6bca06b3c4a1456deafda31fb46d74652`
- pair sync immediately before drain:
  `0x11842a615f69bbe76cf1942a34af4e31afb226eb59162799ce9894f0d3ea5aee`
- drain sell through Uniswap V2 router:
  `0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697`
- creator/drain actor observed in the local trace:
  `0x3b6a5e29d68251265d5ca522f79caad95392b47b`

## Network Replay Baseline

Initial run:

```bash
cargo run -p eth_token --example token_network_flow_context -- \
  --token 0x8CEDa8619ad186E7C9bCA77734e48c8f518f5d01 \
  --start 25077304 \
  --end 25081742 \
  --max-token-blocks 256 \
  --max-seeds 16 \
  --max-blocks-per-address 64 \
  --lookback 300 \
  --lookahead 80 \
  --top 30
```

Observed summary:

```text
token_blocks:      114
graph_nodes:       863
graph_edges:       2691
graph_addresses:   234
context_blocks:    155
flow_seeds:        16
flow_nodes:        21
flow_edges:        53
flow_clusters:     3
```

Top-level interpretation:

- The first-order token network is large for a short activity window.
- The pool/router edges dominate raw WETH movement and need protocol labeling.
- The more useful second-order signal is the EOA shared-sink cluster.
- Shared sink `0x50995f97f63f8ec9a131272269f1388d88a03990` had 11 members in
  the initial flow-context run.

## Study Questions

- Which first-order token-network addresses are real wallets versus routers,
  pools, relayers, or helper contracts?
- Which EOAs are linked by direct ETH/WETH funding before pool interaction?
- Did the shared-sink cluster participate in fake volume, early buys, exits, or
  settlement after the drain?
- Which protocol-address labels should be added before this signal is used for
  risk scoring?
- What compact detector output should `eth_chain_server` expose for this case?

## Artifact Plan

- `chain_truth/`: block timestamps, creation tx, pair identity, actor code kind.
- `pool_reserves/`: reserve timeline and peak-to-drain calculations.
- `token_network/`: first-order token graph summaries and selected seed list.
- `flow_context/`: fund-flow observations, clusters, and protocol-suppression
  notes.
- `txs/`: decoded key transaction receipts/logs/traces.
