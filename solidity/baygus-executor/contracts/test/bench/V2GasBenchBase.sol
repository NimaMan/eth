// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusTradingVault} from "../../src/BaygusTradingVault.sol";
import {IERC20} from "../../src/interfaces/IERC20.sol";
import {IUniswapV2Router02} from "../../src/interfaces/IUniswapV2Router02.sol";
import {TestBase} from "../utils/TestBase.sol";

abstract contract V2GasBenchBase is TestBase {
    address internal constant OWNER = address(0xA11CE);
    address internal constant TREASURY = address(0xBEEF);
    uint256 internal constant BUY_VALUE = 1 ether;
    uint256 internal constant TOKEN_AMOUNT = 100 ether;
    uint256 internal constant DEADLINE = 2_000_000_000;

    function _path(address tokenIn, address tokenOut) internal pure returns (address[] memory path) {
        path = new address[](2);
        path[0] = tokenIn;
        path[1] = tokenOut;
    }

    function _directBuy(address router, address weth, address token, address recipient, uint256 value, uint256 minOut)
        internal
    {
        IUniswapV2Router02(router)
        .swapExactETHForTokensSupportingFeeOnTransferTokens{
            value: value
        }(minOut, _path(weth, token), recipient, DEADLINE);
    }

    function _directSell(
        address router,
        address token,
        address weth,
        address recipient,
        uint256 amountIn,
        uint256 minOut
    ) internal {
        IUniswapV2Router02(router)
            .swapExactTokensForETHSupportingFeeOnTransferTokens(
                amountIn, minOut, _path(token, weth), recipient, DEADLINE
            );
    }

    function _deployVault(address weth, address router) internal returns (BaygusTradingVault) {
        return new BaygusTradingVault(OWNER, TREASURY, weth, router);
    }

    function _approve(address token, address spender, uint256 amount) internal {
        require(IERC20(token).approve(spender, amount), "approve failed");
    }
}
