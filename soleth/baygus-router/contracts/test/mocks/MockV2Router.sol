// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../src/interfaces/IERC20.sol";

contract MockV2Router {
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address recipient,
        uint256 deadline
    ) external returns (uint256[] memory amounts) {
        // Transfer tokens from msg.sender (BaygusRouter) to this contract
        // In real V2, it transfers to Pair. Here we just take it to simulate consumption.
        IERC20(path[0]).transferFrom(msg.sender, address(this), amountIn);
        
        // Mock return
        amounts = new uint256[](path.length);
        amounts[path.length - 1] = amountOutMin; 
        
        // Mint/Transfer output to recipient (simulate swap)
        // For simplicity, we just assume we have output token or mint it if it's MockERC20
        // But MockERC20 mint is external. 
        // Let's just transfer if we have balance, or fail. 
        // To make it easier, we won't actually send output tokens unless pre-funded.
        // We just verify the input transfer happened.
        return amounts;
    }
}