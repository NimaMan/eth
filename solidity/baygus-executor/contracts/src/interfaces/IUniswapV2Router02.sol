// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IUniswapV2Router02 {
    // forge-lint: disable-next-line(mixed-case-function)
    function swapExactETHForTokensSupportingFeeOnTransferTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external payable;

    // forge-lint: disable-next-line(mixed-case-function)
    function swapExactTokensForETHSupportingFeeOnTransferTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external;
}
