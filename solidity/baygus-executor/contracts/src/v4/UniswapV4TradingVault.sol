// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @notice Assessment-only scaffold for the future Uniswap V4 trading vault.
/// @dev This contract is intentionally undeployable. Replace this with the
/// reviewed implementation only after V4 simulator, fork, gas, and policy
/// evidence exists under onchain-deployments/uniswap-v4-trading-vault.
contract UniswapV4TradingVault {
    error AssessmentOnly();

    constructor() {
        revert AssessmentOnly();
    }
}
