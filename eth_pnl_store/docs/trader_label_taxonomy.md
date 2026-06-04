# Trader Label Taxonomy

**Purpose.** Define a deterministic taxonomy that classifies every address ("trader")
in a `token_pnl` run into exactly one primary label plus orthogonal validity flags,
so that address-level PnL can be filtered to the subset that is trustworthy as
trader edge. The unit of analysis is an **address aggregated across all its pools**,
not a single pool row. This taxonomy is calibrated against run
`token-pnl-semantic-traders-50k-20260602-01` (blocks 25180546..=25230545, ~7 days),
where the top-100-by-PnL cohort contains **zero** clean spot traders.

This is the Tier-One backtest-validity blocker: until each address can be labeled
and the infra/scam/contradiction classes are excluded, the leaderboard is not a
trader-edge signal.

---

## Why these signals

PnL rows are pool-scoped: `realized_pnl` is net denom (WETH/ETH) cashflow on closed
positions; an open token balance on a terminal/scam pool is valued at zero. Several
deterministic facts about the *actor* determine whether that cashflow is one
trader's edge or an artifact:

- **Entity type** (`eth_getCode`): EOA (`0x`), EIP-7702 delegated EOA
  (`0xef0100` + 20-byte delegate), or contract (`>0` bytecode). A contract's PnL is
  aggregated flow, never one trader.
- **Gas source**: `gas_paid>0` and `tx.from == address` means the address is its own
  economic fee source. `gas_paid==0` / `tx.from != address` means it is only a
  recipient leg or an operator-paid contract.
- **Role set** (`actor_roles`): `token_creator`, `pool_creator`, `seller`,
  `external_token_source`, `user_candidate`, `custody_victim_candidate`.
- **Selector hints** in bytecode: `uniswapV3SwapCallback` = `fa461e33`,
  `pancakeV3SwapCallback` = `23a69e75`, `uniswapV2Call` = `10d1e85c`. Any present =
  automated swap-callback bot.
- **Scam-pool PnL share**: fraction of aggregated PnL earned on `is_scam=true` pools.
- **Pool / movement counts** and **reconciliation_status** (`movement_backed` /
  `partial_movements` / `aggregate_only`; `movement_rows_retained` vs `movement_count`).
- **Registry match**: `address_book.rs` (routers, Universal Router) and
  `discover_validators.rs` (builders).

---

## Decision order (classifier applies top-down, first match wins)

1. **Registry** — if the address matches `address_book.rs` (router/Universal
   Router/known bot) -> `infra_router_or_bot`; if it matches `discover_validators.rs`
   (builder/validator fee recipient) -> `infra_mev_builder`; if it is a canonical
   protocol contract (WETH, factory, NonfungiblePositionManager, CoW GPv2Settlement)
   -> `infra_protocol`. **Exclude.**
2. **Bytecode / selectors** — if `eth_getCode` is a contract:
   - bytecode contains `fa461e33` / `23a69e75` / `10d1e85c` -> `mev_arb_bot_contract`.
   - no callback selector but many distinct senders route through it (or it sits as
     an inner leg of a known router) -> `infra_router_or_bot`.
   - bytecode `<= 60` bytes (clone/stub/self-destructed) -> `minimal_proxy_contract`.
   - otherwise -> `contract_unknown` (drill: code size, does it route for many
     distinct senders?).
   - EIP-7702 (`0xef0100...`) is **not** a contract here — treat as an EOA and fall
     through to roles.
3. **Roles** — EOA that is `token_creator` AND `pool_creator` of the pools it nets ETH
   from (its own drained/rugged pools) -> `creator_scammer_eoa`. A small contract that
   creates token+pool and seizes ETH -> `scam_factory_contract`.
4. **Gas source + cashflow shape** — for remaining EOAs:
   - only `native_denom_in` legs (receives ETH, never buys/sells a token),
     `gas_paid==0`, off-pool inflow -> `external_inflow_seller_eoa` (receive-only
     settlement/collection variant).
   - `external_token_source` + `seller`, off-pool token inflow then sells, regardless
     of gas source -> `external_inflow_seller_eoa`.
   - buys then sells a fresh (usually scam) token and exits ->
     `fresh_launch_sniper_eoa`.
   - `custody_victim_candidate` role with **positive** PnL -> `custody_anomaly_eoa`
     (then verify: is it the seizer/creator, a sniper that escaped, or a genuine
     accounting contradiction? — see the flag).
5. **Scam share** — only after the above is an EOA eligible for
   `clean_spot_trader_eoa`: it must buy/sell on **non-scam** pools, pay its own gas,
   and have few pools. In this run that set is empty.

---

## Primary labels

### INFRASTRUCTURE (exclude — PnL is meaningless as one trader)

#### `infra_router_or_bot`
Known router / Telegram-bot contract, or a contract that aggregates flow from many
distinct sender EOAs into one/few shared routers (Banana Gun, Maestro, Unibot,
Universal Router, 1inch, MetaMask, LiFi, ERC-4337 bundlers).
- **Signals**: entity = contract or registry match; bytecode may lack a callback
  selector; tx tracing shows 5-12 **distinct** `tx.from` senders routing to one/few
  shared routers; `is_user_candidate=true` (the calc bug); `gas_paid==0`.
- **PnL**: **exclude.** Aggregated multi-user router flow.
- **Examples**: `0x35fC556d6f8675b26FDf1542e6E894100155b34e` (rank 1, 23.8 KB
  Banana-Gun-style router, 6 distinct senders -> shared router
  `0x3328f7f4a1d1c57c35df56bbf0c9dcafca309c49`);
  `0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad` (rank 58, registry match
  UniswapUniversalRouter, 3 distinct routed senders).

#### `infra_mev_builder`
Block builder / validator fee recipient.
- **Signals**: registry match in `discover_validators.rs`; often `code_bytes=0` (EOA
  fee recipient); all movements `native_denom_in` (block-reward/fee aggregation);
  `gas_paid==0`.
- **PnL**: **exclude.**
- **Example**: `0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97` (rank 68, "Titan Builder",
  94 pools, all 1827 movements `native_denom_in`).

#### `infra_protocol`
Canonical protocol contract: WETH, factories, NonfungiblePositionManager, batch
settlement.
- **Signals**: registry / known-address match; large protocol bytecode.
- **PnL**: **exclude.**
- **Example**: `0x9008d19f58aabd9ed0d60971565aa8510560ab41` (rank 40, canonical CoW
  Protocol GPv2Settlement, 16165 bytes; its +9.09 ETH is batched solver flow).

### AUTOMATED CONTRACTS

#### `mev_arb_bot_contract`
Contract exposing a swap-callback selector; automated arb/sandwich/MEV flow, gas paid
by an operator EOA.
- **Signals**: bytecode contains `fa461e33` / `23a69e75` / `10d1e85c`; many
  pools/movements or multi-pool offsetting legs; `gas_paid==0`;
  `is_user_candidate=true` (calc bug). Operator can be single (e.g. `0xfeefee6e...`)
  or many distinct searchers (shared MEV infra).
- **PnL**: **exclude** (or **suspect** where verdict not yet firmed). Bot flow, not a
  trader.
- **Examples**: `0x71eA8223a24B82456F22716f787219bC15Db812B` (rank 2, 23.3 KB,
  `fa461e33`+`23a69e75`+withdraw, 132 pools/7708 movements);
  `0xcaDac8198478AB8341a7a22Ea16bf315b38981eB` (rank 66, `fa461e33`+`23a69e75`,
  routes to gas-golfed MEV bots `0x000000003d55`/`0x0000000025e9`).

#### `contract_unknown`
Contract with no callback selector, not in the registry — custom bot / smart wallet /
proxy.
- **Signals**: `>0` bytecode, no callback selector; drill code size and whether it
  routes for many distinct senders. If single-operator-driven and 100% scam, it is a
  custom bot.
- **PnL**: **suspect.**
- **Example**: `0xe540eb6bfee129d28d47e26ad33a138d66fd78f5` (rank 59, 23102 bytes, no
  `fa461e33`/`10d1e85c`, both main pools driven by single operator `0x555ce236`,
  100% scam).

#### `minimal_proxy_contract`
Tiny bytecode (`<= 60` bytes) clone/stub/self-destructed contract.
- **Signals**: `eth_getCode` length `<= 60` bytes.
- **PnL**: **exclude / suspect.**
- **Examples**: none in the top-100 cohort (provisional minimal-proxy labels at ranks
  91/95 were corrected to EIP-7702 EOAs on `eth_getCode`).

### SCAM / CREATOR ACTORS

#### `scam_factory_contract`
Small contract that creates token+pool and seizes ETH (templated rug); roles
`token_creator`+`pool_creator`+`seller`.
- **Signals**: small contract bytecode (`>0`); `token_creator==pool_creator==self`;
  `is_scam` pool.
- **PnL**: **exclude / suspect** (rug proceeds, not edge).
- **Note**: in this run the provisional `scam_factory_contract` rows (ranks 43-50, 90)
  were all corrected to `creator_scammer_eoa` — on-chain code is the EIP-7702
  designator `0xef0100` + MetaMask DeleGator impl `0x63c0c19a282a1b52b07dd5a65b58948a07dae32b`,
  i.e. delegated **EOAs**, not factory contracts.

#### `creator_scammer_eoa`
EOA (incl. EIP-7702 delegated EOA) that creates token/pool and nets ETH from its own
(usually drained) pool.
- **Signals**: entity EOA / `0xef0100`; `token_creator==pool_creator==self`
  (`creator_match=true`); `is_scam` `liquidity_removed` / `reserve_dump_drain` /
  `backdoor_drain`; pays its own gas (`tx.from==self` -> UniV2 Router); seeds then
  reserve-dumps its own pool.
- **PnL**: **suspect** (rug extraction is real cashflow but not trader edge); **exclude**
  where the seize is a one-block LP pull.
- **Examples**: `0xae4b3464df763235f07bd151059db11758219f10` (rank 43, EIP-7702
  DeleGator, seeds 10 WETH and pulls 17.77 WETH = 7.77 net on its own
  liquidity_removed pool); `0xd1d3a949a8c28139693ec4e0e2fa148c5ec10930` (rank 96,
  pulls 3.2999 ETH out of its own `0xdd22d2c8` LP-removal rug in one block).

### TRADER EOAs

#### `fresh_launch_sniper_eoa`
EOA buying newly-launched (often scam) tokens early and exiting; PnL concentrated on
scam / `liquidity_removed` pools.
- **Signals**: entity EOA; **has both buy (`denom_out`/`token_in`) and sell
  (`token_out`/`native_denom_in`) legs**; buys then sells before the rug/seize;
  scam-dominated PnL. May carry `custody_victim_candidate` if it bought a confiscation
  rug — but it escaped pre-seize, so the victim tag is a buyer heuristic.
- **PnL**: **needs_review** (genuine exit cashflow on scam pools) to **exclude** (when
  100% scam).
- **Examples**: `0x1f0e02b29892150bd6dfd14ee38b1d39b48b6a66` (rank 80, 11 buys 4.1 ETH
  out then 17-18 sells 7.48 ETH in); `0x96212e2d128595f94a7a4624dcb1d4d6e77b9016`
  (rank 91, bought+sold for +3.51 ETH before the seize on rug `0x12d44f8b`).

#### `external_inflow_seller_eoa`
EOA whose tokens arrive off-pool (`external_token_source`) and which only sells
(airdrop/claim dumper, team/distribution wallet) — **or** the recipient-only variant
that only receives ETH off-pool (a bot's settlement/collection EOA).
- **Signals**: entity EOA; roles `external_token_source`+`seller`, or **only
  `native_denom_in` legs** with zero token in/out and zero `native_denom_out`
  (recipient-only); `pool_direct=false`; gas source varies (self-paid sellers vs
  `gas_paid==0` recipient legs); usually 100% scam.
- **PnL**: **needs_review** to **exclude** (it is recipient/dump cashflow, not trading
  edge; never a clean trader).
- **Examples**: `0x9ccF7d800cF38aC9a597a6bff2a083B01cccaa3A` (rank 7, self-paid,
  `external_token_source`+`seller`, 553 `native_denom_in` + 52 `token_out`);
  `0x36c11106814ef31b14eeb42391c39c95917f6019` (rank 21, recipient-only: 207 legs all
  `native_denom_in` +98.76 ETH, zero token legs, `gas_paid==0`).

#### `clean_spot_trader_eoa`
EOA buying/selling on **non-scam** pools, pays its own gas, few pools — the only
"trustworthy good trader" class.
- **Signals**: entity EOA; own fee source; buy+sell legs on `is_scam=false` pools;
  low pool count; scam-share well below 80%.
- **PnL**: **likely_trustworthy.**
- **Examples**: **none in this run's top-100.** This class is the validation target;
  its absence at the top is the headline finding.

#### `custody_anomaly_eoa`
EOA flagged `custody_victim_candidate` yet showing **positive** PnL — an accounting
contradiction.
- **Signals**: entity EOA; `custody_victim_candidate` role on a
  `custody_buyer_token_confiscation` honeypot (`can_buy=can_sell=false`); positive
  total PnL; frequently `aggregate_only` with `movement_rows_retained=0` or
  inflow-only retained legs.
- **PnL**: **suspect** — untrustworthy. Resolve into one of: seizer/creator (PnL
  correct, victim role mis-applied), sniper that escaped (relabel
  `fresh_launch_sniper_eoa`), or a genuine reconciliation calc bug (buy-side outflow
  dropped).
- **Examples**: `0x5ece2eee00d6b1003729e477321b2ee3e520ea66` (rank 38, honeypot,
  +22.42 ETH on `0x12d44f`, 0 movement rows persisted — unauditable);
  `0x74c10E4bbe847D68cE02a9ABB4baB8DbedFD4675` (rank 70, only inflow legs retained,
  buy-side outflow dropped -> impossible +4.87 ETH).

---

## Validity flags (orthogonal, may stack)

#### `infra_mislabeled_as_user`
`is_user_candidate=true` on an infra/bot contract (exclusion list not applied) —
**CALC BUG**.
- **Confirm when**: `is_user_candidate=true` AND entity is a `>0`-byte contract that is
  a registry router, a swap-callback bot, or aggregates many distinct senders.
- **Prevalence**: 40 of the top-100. Confirmed on every router/bot contract drilled
  (e.g. ranks 1, 11, 22, 58, 61, 63, 64, 66, 67).

#### `gas_not_attributed_to_address`
Address pays 0 gas; not the economic fee source.
- **Confirm when**: DB `gas_paid==0` AND sampled `tx.from != address` (a different EOA
  or operator paid). **Remove** when `tx.from==self` and `native_fee>0` (own fee source).
- **Prevalence**: 58 of the top-100. Correctly absent on self-paid sellers (ranks 7,
  13, 16, 19, 26) and creator-scammers paying their own gas.

#### `pnl_dominated_by_scam_pools`
`>= 80%` of PnL from `is_scam` pools; "profit" is rug cashflow.
- **Confirm when**: computed scam-pool PnL share `>= 0.80` (hard threshold).
- **Remove when below 0.80**: corrected off rank 31 (45.9%), 34 (49.9%), 54 (77.3%),
  56 (72.6%), 71 (71.9%); correctly omitted on rank 15 (73.2%), rank 30 (47.3%). One
  retained-but-borderline case noted: rank 5 at 0.687 (excluded as infra regardless).
- **Prevalence**: 91 of the top-100.

#### `movement_reconciliation_incomplete`
Partial/aggregate movement backing; retained rows don't cover the aggregate counters.
- **Confirm when**: `reconciliation_status` is `partial_movements` or `aggregate_only`,
  AND `movement_rows_retained < movement_count`. **Absent** when fully `movement_backed`
  (`retained==counted`, e.g. rank 11: 1663==1663).
- **Prevalence**: 62 of the top-100. Worst cases: `aggregate_only` with 0 rows
  persisted (ranks 38, 86, 93, 97, 98) make the PnL wholly unauditable.

#### `custody_victim_with_positive_pnl`
Labeled `custody_victim_candidate` but profitable — contradiction / calc-bug candidate.
- **Confirm when**: `custody_victim_candidate` role present AND total PnL positive.
  Then triage: (a) genuine calc bug if buy-side legs are dropped or 0 rows persisted
  (ranks 38, 70); (b) role mis-attribution on the seizer/creator (rank 29); (c)
  settlement-contract-as-victim (rank 40); (d) sniper that escaped (ranks 91, 97, 98,
  100 — relabel `fresh_launch_sniper_eoa`, remove flag intent as a PnL bug). Many
  cohorts carry **no** custody role at all, so the flag is correctly absent there.
- **Prevalence**: 8 of the top-100.

---

## How to apply later (all 20,146 addresses)

1. **Materialize signals once per address** (aggregated across pools): `entity_type`
   from `eth_getCode` (EOA / `0xef0100` EIP-7702 / contract, plus byte length);
   `gas_paid` and a sampled `tx.from==address` test; the union of `actor_roles`;
   bytecode selector membership (`fa461e33`/`23a69e75`/`10d1e85c`); scam-pool PnL
   share; pool count, `movement_count`, `movement_rows_retained`,
   `reconciliation_status`; registry membership (`address_book.rs`,
   `discover_validators.rs`).
2. **Run the Decision order** above to assign exactly one primary label.
3. **Stack the validity flags** independently using their confirm/remove rules
   (enforce the 0.80 scam-share cutoff and the `retained<counted` reconciliation rule
   exactly — do not eyeball).
4. **Map label -> trust tier** for the trader-edge cohort:
   - `infra_*`, `mev_arb_bot_contract`, `scam_factory_contract`, `minimal_proxy_contract`,
     `creator_scammer_eoa` -> **exclude**.
   - `external_inflow_seller_eoa`, `fresh_launch_sniper_eoa`, `custody_anomaly_eoa`,
     `contract_unknown` -> **suspect / needs_review** (never trader edge).
   - `clean_spot_trader_eoa` -> **likely_trustworthy** (the only includable class).
5. **Gate**: only `clean_spot_trader_eoa` rows that pass with no stacked validity flag
   enter the trustworthy trader-PnL set. Apply the same registry/selector exclusion at
   ingest so `is_user_candidate` is never `true` on a contract.
6. **Cheap pre-filters** to scale: registry match and bytecode-length/selector scan are
   O(1) per address and remove the bulk of infra; scam-share `>= 0.80` removes most
   rug cashflow before any RPC drilling is needed.
