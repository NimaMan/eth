# Detector Prototypes

Detector prototypes start here before promotion into production code.

A detector is ready to promote when it has:

- a concrete case that motivated it;
- chain truth showing the behavior;
- simulator parity or a documented simulator limitation;
- thresholds that are expressed in meaningful units;
- low false-positive risk for normal pools;
- a regression test or fixture.

## Range Triage Tool

`range_triage.py` is the first agent-native range triage runner. It inspects an
already completed token range run and reports possible investigation cases. It
does not mutate token state, rerun simulations, write case artifacts, or create
frontend-only labels.

The tool should be easy for an agent to run from a shell and easy to parse:

```text
token_safety_lab/tools/detectors/range_triage.py \
  --api http://127.0.0.1:8765 \
  --run active \
  --format markdown
```

Implementation shape:

- `RangeRunSnapshot`: loads run progress, tokens, pools, errors, and selected
  token/pool details from the token server API.
- `IssueCandidate`: a structured issue record with kind, severity, token,
  pool, protocol, block range, evidence, metrics, and suggested next step.
- `IssueDetector`: a small detector interface. Each detector reads the snapshot
  and returns candidates.
- detector classes: run independent rules over the snapshot and return
  candidates.
- `RangeTriageRunner`: executes selected detectors and ranks candidates.
- output helpers: rank candidates and emit JSON, JSONL, Markdown, or a compact
  terminal table.

The code lives under this folder because the rules are investigation rules, not
production safety rules. Once a detector is proven by cases and regression
coverage, promote the stable part into Rust crate logic or an `eth_token_server`
triage endpoint.

## Candidate Issue Families

These are the first issue families the range triage runner should identify from
run cache data.

| Family | Candidate Signals | Why It Matters |
| --- | --- | --- |
| simulator parity | `can_buy=true` with `can_sell=false`, missing failure class, unexpected failure reason, observed sell evidence with simulated sell failure | trading decisions must match chain behavior or have a clear route/state explanation |
| route mismatch | Universal Router, Permit2, aggregator, or non-classic-router activity around a pool that fails classic V2 simulation | a classic-router simulator can create false `cannot_sell` labels for route-dependent tokens |
| amount/dust parity | `INSUFFICIENT_INPUT_AMOUNT`, zero buy output, tiny simulated sell input, dust token reserve, dust token supply in pool | dust or amount extraction bugs can make a pool appear buyable while the sell path is meaningless |
| extreme price ratio | huge price ratio with low liquidity, tiny token reserve, or near-zero token supply in pool | price ratios are not useful when driven by dust or drained liquidity |
| liquidity drain | meaningful liquidity deposit followed by denom reserve collapse, very low liquidity, or liquidity/FDV near zero | drained pools should not look like active healthy pools |
| LP control | concentrated LP holders, high approved LP share, router or non-router approvals, zero LP supply with reserves | LP ownership and approvals determine whether liquidity can be removed |
| tax safety | no, low, moderate, high, and extreme tax buckets from `eth_token::pools::TaxBucket` | tax is a trading safety input and should be grouped consistently across pages and tools |
| lifecycle consistency | token without a pool showing pool-derived trading state, pool with liquidity but no reliable trading simulation, trading state before pool creation | token-level labels must not invent pool state |
| token supply anomalies | balances or pool supply share over total supply, hidden mint indicators, supply in pool over 100% | supply math errors and hidden mints can invalidate liquidity and FDV metrics |
| protocol coverage | V2, V3, and V4 pools missing protocol-specific metrics or showing V2-only assumptions | the triage runner must stay protocol-aware as V3 and V4 enter the range builds |
| server/indexer errors | block application errors, simulator errors, metadata soft misses that are not expected, timeout clusters | infrastructure failures should be separated from token behavior |

## Output Contract

Each candidate should include enough data for the next agent step:

```json
{
  "kind": "simulator_parity.cannot_sell",
  "severity": "high",
  "status": "new",
  "token_address": "0x...",
  "pool_address": "0x...",
  "protocol": "uniswap_v2",
  "symbol": "TOKEN",
  "range_start": 24987948,
  "range_end": 24987962,
  "evidence": [
    "can_buy=true",
    "can_sell=false",
    "last_trading_failure_class=transfer_from_failed"
  ],
  "metrics": {
    "liquidity_weth": 1.23,
    "price_ratio_initial": 44.1
  },
  "suggested_next_step": "check observed sells and replay the actual route before calling this a honeypot"
}
```

Markdown output should be suitable for pasting into `cases/README.md`, but the
tool should not edit that file by default.
