# MOTH Mothman Live Retention Liquidity Removal

This investigation tracks a live token retention policy mismatch where Asena
showed a token as `PAIR_CREATION`, `risk=clear`, and `pools=0` while the
mempool signal page showed a confirmed high-drain liquidity-removal signal for
the same token.

## Addresses

- token: `0xec402ba62e9d67650359ba859ddc1ae22a84840f`
- pool: `0x43cc493e758bfb50ff0f84ca188e2555145bef2d`
- denom: `WETH`
- creator: `0x6c5d2bdf75268108469f1a8659a1db7ac85bad51`

## Range

- token creation block: `25,063,339`
- liquidity-removal block: `25,063,350`
- live tracker observed later token activity through at least block `25,063,389`

## Key Transactions

- `token_creation`:
  `0x50399c5e562a5ef8d5943acfb034d6a7f42a97f4af4770449f02322f99f870a0`
- `liquidity_removal`:
  `0x84b4c5eca672c72363048b0b49f47cdfb15bda3953e69138b0252e0c76437510`

## Observed Mismatch

The live token detail API had token activity for the removal transaction and
swap activity around the pool, but the token had no pool views:

```text
token_life_cycle_status: PAIR_CREATION
pool_count: 0
pools: []
activity includes tx 0x84b4...7510 at block 25,063,350
```

The mempool signal API had the matching pool and high-drain removal:

```text
signal_id: 1724
pool: 0x43cc493e758bfb50ff0f84ca188e2555145bef2d
removed_eth: 1.246518636309431600
remaining_eth: 0.019610863690568303
removal_percentage: 98.45
risk: DRAINING
```

## Root Cause

Live retention used the current WETH reserve floor (`0.1 ETH`) as a pruning
rule. After the removal, the pool reserve fell to about `0.0196 ETH`, so the
pool was removed from the live registry. Immediate removal is correct for
keeping the live set small, but it leaves no inspection window for recent scam
tokens on the token detail page. The token lifecycle also stayed stale as
`PAIR_CREATION` after the last pool was pruned.

## Fix

The live retention path now preserves evidence-bearing depleted pools for a
bounded inspection window:

- if a pool falls from above the retention threshold to below it by at least
  `80%`, retention marks the pool as `liquidity_removal`;
- marked liquidity-removal pools are retained for `15,000` blocks even when
  current liquidity is below the live retention floor, then removed;
- token lifecycle is refreshed after retention pruning, so stale
  `PAIR_CREATION` does not remain after all pools are removed;
- token risk and scam label now include pool liquidity-removal evidence while
  hidden-mint status remains separately classified.

## Regression

Added a focused `eth_token` unit test that mirrors this case:

```text
WETH reserve: 1.2661295 -> 0.0196108
expected: pool retained during the 15,000-block window, marked liquidity_removal,
then pruned after the window expires
```

## Follow-up

After deploying the fix, restart or re-warm the live token tracker. The currently
running process has already pruned this specific pool from memory, so it needs a
fresh replay window to reconstruct the retained pool state.
