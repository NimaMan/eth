// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../../src/interfaces/IERC20.sol";
import {UniswapV4TradingVault} from "../../../src/v4/UniswapV4TradingVault.sol";
import {TestBase} from "../../utils/TestBase.sol";
import {MockPermit2} from "../../v4/mocks/MockPermit2.sol";
import {MockUniversalRouterV4} from "../../v4/mocks/MockUniversalRouterV4.sol";

abstract contract V4GasBenchBase is TestBase {
    address internal constant OWNER = address(0xA11CE);
    address internal constant TREASURY = address(0xBEEF);
    uint128 internal constant TOKEN_AMOUNT = 2_000e6;
    uint128 internal constant MIN_TOKEN_OUT = 2_000e6;
    uint128 internal constant MIN_ETH_OUT = 0.9 ether;
    uint128 internal constant BUY_VALUE = 1 ether;
    uint256 internal constant DEADLINE = 2_000_000_000;
    uint48 internal constant PERMIT2_EXPIRATION = type(uint48).max;

    uint8 private constant COMMAND_V4_SWAP = 0x10;
    uint8 private constant ACTION_SWAP_EXACT_IN_SINGLE = 0x06;
    uint8 private constant ACTION_SETTLE = 0x0b;
    uint8 private constant ACTION_TAKE = 0x0e;

    function _deployVault(address router, address permit2) internal returns (UniswapV4TradingVault) {
        return new UniswapV4TradingVault(OWNER, TREASURY, router, permit2, false);
    }

    function _directBuy(
        MockUniversalRouterV4 router,
        MockUniversalRouterV4.PoolKey memory key,
        address tokenOut,
        address recipient,
        uint128 value,
        uint128 minOut
    ) internal {
        router.execute{
            value: value
        }(
            abi.encodePacked(COMMAND_V4_SWAP),
            _inputs(key, true, address(0), tokenOut, value, minOut, recipient, ""),
            DEADLINE
        );
    }

    function _directSell(
        MockUniversalRouterV4 router,
        MockUniversalRouterV4.PoolKey memory key,
        address tokenIn,
        address recipient,
        uint128 amountIn,
        uint128 minOut
    ) internal {
        router.execute(
            abi.encodePacked(COMMAND_V4_SWAP),
            _inputs(key, false, tokenIn, address(0), amountIn, minOut, recipient, ""),
            DEADLINE
        );
    }

    function _approveErc20ToPermit2(address token, address permit2, uint256 amount) internal {
        require(IERC20(token).approve(permit2, amount), "approve failed");
    }

    function _approvePermit2ToRouter(MockPermit2 permit2, address token, address router, uint160 amount) internal {
        permit2.approve(token, router, amount, PERMIT2_EXPIRATION);
    }

    function _inputs(
        MockUniversalRouterV4.PoolKey memory key,
        bool zeroForOne,
        address tokenIn,
        address tokenOut,
        uint128 amountIn,
        uint128 minOut,
        address recipient,
        bytes memory hookData
    ) private pure returns (bytes[] memory inputs) {
        bytes memory actions = abi.encodePacked(
            bytes1(ACTION_SWAP_EXACT_IN_SINGLE), bytes1(ACTION_SETTLE), bytes1(ACTION_TAKE)
        );
        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(
            MockUniversalRouterV4.ExactInputSingleParams({
                poolKey: key,
                zeroForOne: zeroForOne,
                amountIn: amountIn,
                amountOutMinimum: minOut,
                hookData: hookData,
                minHopPriceX36: 0
            })
        );
        params[1] = abi.encode(tokenIn, amountIn, true);
        params[2] = abi.encode(tokenOut, recipient, uint256(0));

        inputs = new bytes[](1);
        inputs[0] = abi.encode(actions, params);
    }
}
