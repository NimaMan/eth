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
        deadline;
        require(IERC20(path[0]).transferFrom(msg.sender, address(this), amountIn), "V2 input transfer");

        uint256 amountOut = amountOutMin == 0 ? amountIn : amountOutMin;
        require(IERC20(path[path.length - 1]).transfer(recipient, amountOut), "V2 output transfer");

        amounts = new uint256[](path.length);
        amounts[0] = amountIn;
        amounts[path.length - 1] = amountOut;
    }
}
