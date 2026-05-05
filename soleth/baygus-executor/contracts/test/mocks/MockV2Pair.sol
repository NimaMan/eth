// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../src/interfaces/IERC20.sol";

contract MockV2Pair {
    address public immutable token0;
    address public immutable token1;

    constructor(address token0_, address token1_) {
        token0 = token0_;
        token1 = token1_;
    }

    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external {
        data;
        require(amount0Out == 0 || amount1Out == 0, "MockV2Pair: one side");
        if (amount0Out != 0) {
            require(IERC20(token0).transfer(to, amount0Out), "MockV2Pair: token0 transfer");
        }
        if (amount1Out != 0) {
            require(IERC20(token1).transfer(to, amount1Out), "MockV2Pair: token1 transfer");
        }
    }
}
