// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../src/interfaces/IERC20.sol";

contract MockV2Router {
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable weth;

    uint256 public nextTokenOut;
    uint256 public nextEthOut;
    uint256 public lastEthIn;
    uint256 public lastTokenIn;
    address public lastRecipient;

    constructor(address weth_) {
        weth = weth_;
    }

    receive() external payable {}

    function setNextTokenOut(uint256 amount) external {
        nextTokenOut = amount;
    }

    function setNextEthOut(uint256 amount) external {
        nextEthOut = amount;
    }

    // forge-lint: disable-next-line(mixed-case-function)
    function swapExactETHForTokensSupportingFeeOnTransferTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external payable {
        require(block.timestamp <= deadline, "MockV2Router: expired");
        require(path.length == 2, "MockV2Router: path");
        require(path[0] == weth, "MockV2Router: weth path");
        require(msg.value > 0, "MockV2Router: no eth");

        uint256 amountOut = nextTokenOut == 0 ? msg.value : nextTokenOut;
        require(amountOut >= amountOutMin, "MockV2Router: token slippage");
        lastEthIn = msg.value;
        lastRecipient = to;

        require(IERC20(path[1]).transfer(to, amountOut), "MockV2Router: token transfer");
    }

    // forge-lint: disable-next-line(mixed-case-function)
    function swapExactTokensForETHSupportingFeeOnTransferTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external {
        require(block.timestamp <= deadline, "MockV2Router: expired");
        require(path.length == 2, "MockV2Router: path");
        require(path[1] == weth, "MockV2Router: weth path");
        require(amountIn > 0, "MockV2Router: no token");

        uint256 amountOut = nextEthOut == 0 ? amountIn : nextEthOut;
        require(amountOut >= amountOutMin, "MockV2Router: eth slippage");
        lastTokenIn = amountIn;
        lastRecipient = to;

        require(IERC20(path[0]).transferFrom(msg.sender, address(this), amountIn), "MockV2Router: token transfer");
        (bool success,) = to.call{value: amountOut}("");
        require(success, "MockV2Router: eth transfer");
    }
}
