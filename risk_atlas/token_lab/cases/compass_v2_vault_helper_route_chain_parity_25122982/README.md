# Compass V2 Vault Helper-Route Chain Parity

Status: confirmed source/simulator/chain parity failure.

This is a focused companion note for
`../external_router_classic_v2_scope_25122982/`. It owns the chain-parity
question for the Compass Uniswap V2 pool at block `25,122,982`: whether the
deployed Alpha V2 vault route is execution-equivalent to the helper-router
routes that successfully sold on chain.

## Coordinates

| Item | Value |
| --- | --- |
| Token | `Compass` / `0xc38a7a4b93bd2112a021c1b55756e5d1805cd237` |
| Pool | `0x0d3d3a33c430f3d43d561551f26a7b7114a93e30` |
| Block | `25,122,982` |
| Deployed V2 vault | `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` |
| Vault owner | `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27` |
| Vault source | `solidity/baygus-executor/contracts/src/UniswapV2TradingVault.sol` |
| Parent investigation | `risk_atlas/token_lab/cases/external_router_classic_v2_scope_25122982/` |

## Conclusion

The deployed Uniswap V2 vault does not close this parity gap by itself. The
successful chain sells and the currently executable Alpha sell path are
different routes.

At block `25,122,982`, the Compass pool was liquid with about
`1.8794457585863258 WETH` in reserve, and three same-block external/helper
router sells extracted `0.8245830985731981 WETH` total. That proves the token
could be sold on chain by those helper routes.

Alpha's current classic V2 sell route fails for real pre-block holders and for
a vault-shaped seller with `TransferHelper: TRANSFER_FROM_FAILED`. The deployed
V2 vault's emergency sell is also classic-router shaped: it approves the
Uniswap V2 router and calls the router as the top-level sell path. That is not
execution-equivalent to the mined helper routes.

This is therefore a source/simulator/chain route-parity problem, not a
strategy-policy-only problem. Until the exact executable route passes at the
historical coordinate, Risk Atlas should treat the helper-route sells as source
evidence only, not as Alpha-executable sellability.

## Observed Failure

The successful helper routes first call `token.transferFrom` and then call
`pair.swap` directly. During `Compass.transferFrom`, the token appears to run a
token-contract self-swap through the Uniswap V2 router before the holder
transfer completes. That ordering matters:

1. Helper route starts with `Compass.transferFrom(holder, helper, amount)`.
2. Compass performs its internal self-swap through the Uniswap V2 router.
3. Helper route computes the received amount and calls `pair.swap` directly.

Alpha's classic V2 route instead calls the Uniswap V2 router as the top-level
sell path. The router then calls `token.transferFrom(vault_or_holder, pair,
amount)`. For Compass at the historical coordinate, that path reverts with
`TransferHelper: TRANSFER_FROM_FAILED`.

## Why The Deployed V2 Vault Is Not Enough

The deployed vault is useful execution infrastructure, but its V2 emergency
sell is still a classic-router route. It does not implement the observed helper
pattern of `transferFrom` followed by direct `pair.swap`.

Historical exact-address simulation is also unavailable without a simulator
overlay. `cast code` showed no runtime code at the deployed vault address for
blocks `25,122,981`, `25,122,982`, and `25,129,615`; code exists only at the
current local head. An exact historical deployed-vault rehearsal therefore
needs `tx_simulator` support to inject or deploy the vault runtime into the
historical fork.

The current-state smoke test is not historical evidence. Compass
`buy -> sell` through the exact deployed vault succeeds at local head, but that
does not prove the same route worked at block `25,122,982`. Conversely,
`COMPASS AI` `buy -> sell` through the same deployed vault fails at local head
with `TransferHelper: TRANSFER_FROM_FAILED`, which proves the vault can still
hit token sell restrictions.

## Evidence We Have

| Evidence | Result | Meaning |
| --- | --- | --- |
| Pool reserve at block `25,122,982` | About `1.8794457585863258 WETH` | This was not a dead-pool or dust-only sell case. |
| Same-block observed helper sells | Three sells extracted `0.8245830985731981 WETH` total | Chain truth proves helper-route sellability. |
| Observed helper route shape | `token.transferFrom` before direct `pair.swap` | The mined path differs from Alpha's classic router path. |
| Alpha classic V2 route for real pre-block holders | `TransferHelper: TRANSFER_FROM_FAILED` | Real holder balances do not make the classic route executable. |
| Classic V2 route with seller set to vault | `TransferHelper: TRANSFER_FROM_FAILED` | A vault-shaped sender does not make the route helper-equivalent. |
| Vault source route shape | Approves and calls the Uniswap V2 router | The deployed vault sell path is classic-router shaped. |
| Vault code at historical blocks | No code at `25,122,981`, `25,122,982`, `25,129,615` | Exact historical replay needs a deploy/code overlay. |
| Current-head Compass vault smoke test | Buy succeeds, sell succeeds | Useful smoke only; not evidence for the historical block. |
| Current-head COMPASS AI vault smoke test | Buy succeeds, sell fails | The vault can still hit token transfer restrictions. |

Representative Compass helper sells from the parent investigation:

| Tx | Block | Tx Index | Helper/Target | WETH Out |
| --- | ---: | ---: | --- | ---: |
| `0x23b55d...937e87` | `25,122,982` | `5` | `0xF827725498E6fcF62D331566965F5254bCda081f` | `0.10934270432841361` |
| `0x4b4d99...4dad2` | `25,122,982` | `93` | `0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e` | `0.3883634954551535` |
| `0x3cdcb5...a98580` | `25,122,982` | `98` | `0x3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49` | `0.3268768987896309` |

## Evidence Missing

The missing evidence is executable route parity at the same historical
coordinate. Specifically:

- exact historical V2 vault rehearsal with the deployed vault runtime injected
  or deployed into the block `25,122,982` fork;
- tx-index-aware replay of the observed helper-route transactions from the
  correct same-block prestate, not only block-level replay;
- side-by-side comparison of observed helper route, classic router route, and
  deployed-vault route at the same block plus transaction index coordinate;
- route-specific Risk Atlas fields that preserve observed-route sellability
  separately from Alpha-executable sellability;
- a live-executable helper-route implementation, if Alpha ever wants to use the
  observed helper pattern instead of the classic router or deployed vault path.

## Implementation Plan

1. Add a historical V2 vault rehearsal in `tx_simulator`.

   The simulator needs a code/deploy overlay that can place the
   `UniswapV2TradingVault` runtime at the deployed vault address or deploy the
   same source-shaped contract into a historical fork. The rehearsal should run
   the exact `buyV2ExactEthForTokens` and `emergencySellV2ExactTokensForEth`
   call shapes against Compass at block `25,122,982`, then record whether the
   vault route sells at the historical coordinate.

2. Add tx-index-aware observed-route replay.

   Start from block `25,122,981`, replay the signed block `25,122,982`
   transaction prefix up to the observed helper tx index, replay the observed
   helper-route transaction, and compare pool reserves, logs, WETH out, and
   revert status. Then run Alpha's exact executable route from the same
   coordinate. The replay coordinate must be block plus transaction index.

3. Persist route-specific Risk Atlas fields.

   Keep these fields distinct: `observed_sell_route_class`,
   `observed_sell_tx_index`, `observed_sell_weth_out`,
   `classic_router_route_sellable`, `alpha_v2_vault_route_sellable`, and
   `route_parity_status`. A helper-route sell should not overwrite
   `can_sell` or `effective_can_sell` for Alpha's classic route.

4. Keep Alpha policy conservative until an exact route passes.

   For Compass-like cases, `chain_route_sellable=true` and
   `classic_router_route_sellable=false` can both be true. Strategy eligibility
   must remain false until the deployed vault route passes at the historical
   coordinate, or until Alpha has a separate helper-route executor with its own
   decoder, calldata builder, simulation coverage, gas policy, and live
   execution validation.

## Acceptance Criteria

This parity item can be closed only after one of these is true:

- the exact deployed-vault route, using a historical code/deploy overlay,
  succeeds for Compass at block `25,122,982` with the same route shape Alpha can
  execute; or
- Alpha implements and validates an executable helper route that matches the
  mined helper-route behavior; or
- Risk Atlas permanently labels this case as helper-route-only source evidence
  and keeps Alpha-executable sellability false for the classic V2/vault route.
