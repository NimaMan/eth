Tx Builders — AMM v2/v3 Calldata Constructors

Purpose
- Stateless builders that construct unsigned transactions for AMM interactions:
  - V2/Sushi: swapExactETHForTokens, swapExactTokensForETH, approve
  - V3: exactInputSingle, approve (router spender), and soon: selfPermit + multicall
- No chain reads here; callers must supply addresses and parameters.

Entrypoints
- `amm/v2.rs`: `build_buy_swap_v2(_with_min_out)`, `build_sell_swap_v2(_with_min_out)`, `build_token_to_token_swap_v2(_with_min_out)`, `build_approve_v2`
- `amm/v3.rs`: `build_buy_swap_v3(_with_min_out)`, `build_sell_swap_v3(_with_min_out)`, `build_token_to_token_swap_v3(_with_min_out)`, `build_approve_v3`
- `mod.rs`:
  - Route-aware dispatchers: `build_buy_swap`, `build_sell_swap`, `build_approve_for_route`
  - Token→Token: `build_token_to_token_swap(_with_min_out)`
  - `spender_for_route(&AmmSwapRoute)` returns the router address to approve
  - `build_sell_with_permit(...)` entrypoint routes to V3 permit builder when available

Permit Plan (V3)
- Implement `build_sell_with_self_permit_v3` to encode:
  - `selfPermit(token, value, deadline, v, r, s)` then `exactInputSingle(...)` inside `multicall(bytes[])` of the SwapRouter.
- Current implementation is a placeholder that falls back to a standard sell; once encoding is added, env can use it when `ExecMode::WithPermit` is set.

Usage
- Builders are used by:
  - Strategy/training environments to generate unsigned txs for simulation
  - tx_processor simulators to orchestrate pool viability checks
- All gas/base fee logic and allowance decisions happen in higher layers; builders only assemble calldata.

Roadmap
- Path-aware multi-hop builders: accept explicit paths (e.g., tokenIn → WETH → tokenOut) and, for V3, per-hop fee tiers.
  - API sketch: `build_token_to_token_path_v2(trader, path: [Address; 3], amount_in, amount_out_min, deadline)`
  - This enables robust routing when no direct pool exists.
