// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

library V2MainnetFixtures {
    address internal constant UNISWAP_V2_ROUTER = 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D;
    address internal constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address internal constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address internal constant RFI = 0xA1AFFfE3F4D611d252010E3EAf6f4D77088b0cd7;
    uint256 internal constant BUY_VALUE = 1 ether;
    uint256 internal constant FEE_ON_TRANSFER_BUY_VALUE = 1 ether / 100;
    uint256 internal constant MIN_USDC_OUT = 1;
    uint256 internal constant MIN_RFI_OUT = 1;
    uint256 internal constant MIN_ETH_OUT = 1;
}
