Tx Builders — AMM Calldata Constructors

Purpose
- Stateless builders that construct unsigned transactions for AMM interactions:
  - V2/Sushi: swapExactETHForTokens, swapExactTokensForETH, approve
  - V3: exactInputSingle, approve (router spender), and soon: selfPermit + multicall
  - V4: Universal Router exact-input single-hop swaps plus Permit2 allowance helpers
- No chain reads here; callers must supply addresses and parameters.
- Builders are where route and execution decisions should be finalized. Prefer adding off-chain
  builder logic over adding on-chain branching, discovery, or generic adapter behavior.

Entrypoints
- `protocols/uniswap/v2.rs`: `build_buy_swap_v2(_with_min_out)`, `build_sell_swap_v2(_with_min_out)`, `build_token_to_token_swap_v2(_with_min_out)`, `build_approve_v2`
- `protocols/sushiswap/v2.rs`: Sushi V2 router constants and V2-router wrapper exports
- `protocols/uniswap/v3.rs`: `build_buy_swap_v3(_with_min_out)`, `build_sell_swap_v3(_with_min_out)`, Universal Router v3 helpers, and `build_approve_v3`
- `protocols/uniswap/v4/`: pool-key orientation, ERC20/WETH helpers, and Universal Router v4 builders
- `protocols/permit2.rs`: Permit2 allowance approval builder
- `route_dispatch.rs`:
  - Route-aware dispatchers: `build_buy_swap`, `build_sell_swap`, `build_approve_for_route`
  - Token→Token: `build_token_to_token_swap(_with_min_out)`
  - `spender_for_route(&AmmSwapRoute)` returns the router address to approve
  - `build_sell_with_permit(...)` entrypoint routes to V3 permit builder when available
- `routes.rs`: `AmmSwapRoute`

Permit Plan (V3)
- Implement `build_sell_with_self_permit_v3` to encode:
  - `selfPermit(token, value, deadline, v, r, s)` then `exactInputSingle(...)` inside `multicall(bytes[])` of the SwapRouter.
- Current implementation is a placeholder that falls back to a standard sell; once encoding is added, env can use it when `ExecMode::WithPermit` is set.

Usage
- Builders are used by:
  - Strategy/training environments to generate unsigned txs for simulation
  - tx_processor simulators to orchestrate pool viability checks
- All gas/base fee logic and allowance decisions happen in higher layers; builders only assemble calldata.
- V4 flows use Uniswap's deployed Universal Router and Permit2. The tx builder does not deploy
  local routers or custom executors.

Roadmap
- Path-aware multi-hop builders: accept explicit paths (e.g., tokenIn → WETH → tokenOut) and, for V3, per-hop fee tiers.
  - API sketch: `build_token_to_token_path_v2(trader, path: [Address; 3], amount_in, amount_out_min, deadline)`
  - This enables robust routing when no direct pool exists.
- Gas-first production builders: benchmark route-specific calldata against Universal Router paths
  before promoting new swap modes.
