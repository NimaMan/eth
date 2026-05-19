// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

library V4MainnetFixtures {
    address internal constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address internal constant UNIVERSAL_ROUTER = 0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af;
    address internal constant PERMIT2 = 0x000000000022D473030F116dDEE9F6B43aC78BA3;

    uint24 internal constant ETH_USDC_500_FEE = 500;
    int24 internal constant ETH_USDC_500_TICK_SPACING = 10;
    uint256 internal constant FIXTURE_BLOCK = 25_131_251;
    uint128 internal constant BUY_VALUE = 1 ether;
    uint128 internal constant MIN_USDC_OUT = 1;
    uint128 internal constant MIN_ETH_OUT = 1;
}
