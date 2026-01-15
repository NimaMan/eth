# Tests

## Problem Domain
This directory contains the comprehensive test suite for the Baygus Router. It verifies the correctness of swap logic, error handling, multi-hop execution paths, and protocol integrations using the Foundry testing framework.

## Logic
The tests are structured as Solidity test contracts (`.t.sol`):
*   **BaygusRouter.t.sol**: Focuses on unit testing individual swap commands (V2, V3, V4, Curve, Balancer) and basic router mechanics.
*   **BaygusRouterMultihop.t.sol**: Validates complex chained scenarios, ensuring intermediate balances are handled correctly and final output meets slippage constraints.
*   **Integration**: Relies on both mocks (for isolated logic testing) and potential fork testing (implied by protocol addresses in source) to simulate real-world conditions.
