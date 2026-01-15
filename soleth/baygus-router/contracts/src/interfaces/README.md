# Interfaces

## Problem Domain
This directory defines the external contract interfaces required for the Baygus Router. These interfaces abstract the interaction with various third-party DEX protocols and token standards, allowing the router to compile and interact without needing the full source code of dependencies.

## Logic
The interfaces provide function signatures for:
*   **Core Protocols**: `IPoolManager` (Uniswap V4), `ISwapRouter` (Uniswap V3), `ICurvePool`, `IBalancerVault`.
*   **Standard Standards**: `IERC20` for token transfers and approvals.
*   **Custom Hooks**: `IHookAdapter` allows the Baygus Router to call out to external logic before or after swaps for customized execution flows.
