// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {PoolKey, SwapParams, BalanceDelta} from "../src/types/SharedTypes.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusRouterMultihopTest is TestBase {
    uint256 constant CMD_V4_SWAP = 0x01;

    function _deployEnvironment()
        internal
        returns (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB, MockERC20 tokenC)
    {
        tokenA = new MockERC20("TokenA", "TKNA", 18);
        tokenB = new MockERC20("TokenB", "TKNB", 18);
        tokenC = new MockERC20("TokenC", "TKNC", 18);

        pool = new MockPoolManager();
        router = new BaygusRouter(address(pool));
        pool.setRouter(address(router));

        tokenA.mint(address(this), 1_000_000 ether);
        tokenB.mint(address(pool), 1_000_000 ether);
        tokenC.mint(address(pool), 1_000_000 ether);
    }

    function _poolKey(address c0, address c1) internal pure returns (PoolKey memory) {
        return PoolKey({
            currency0: c0,
            currency1: c1,
            fee: 1_000,
            tickSpacing: 1,
            hooks: address(0)
        });
    }

    function _params() internal pure returns (SwapParams memory) {
        return SwapParams({zeroForOne: false, amountSpecified: 1 ether, sqrtPriceLimitX96: 0});
    }

    function testMultiHopExactInputPath() external {
        (
            BaygusRouter router,
            MockPoolManager pool,
            MockERC20 tokenA,
            MockERC20 tokenB,
            MockERC20 tokenC
        ) = _deployEnvironment();

        // Hop 1: spend TokenA, receive TokenB
        pool.queueDelta(int128(int256(800 ether)), int128(int256(-500 ether)));
        // Hop 2: spend TokenB, receive TokenC
        pool.queueDelta(int128(int256(600 ether)), int128(int256(-800 ether)));

        tokenA.approve(address(router), type(uint256).max);

        BaygusRouter.Hop[] memory hops = new BaygusRouter.Hop[](2);
        hops[0] = BaygusRouter.Hop({
            key: _poolKey(address(tokenB), address(tokenA)),
            params: _params(),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        hops[1] = BaygusRouter.Hop({
            key: _poolKey(address(tokenC), address(tokenB)),
            params: _params(),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });

        bytes memory innerPayload = abi.encode(
            address(this),
            address(this),
            int128(int256(600 ether)),
            int128(0),
            hops
        );
        bytes memory input = abi.encode(uint8(1), innerPayload);
        
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = input;
        bytes memory commands = abi.encodePacked(uint8(CMD_V4_SWAP));

        router.execute(commands, inputs);

        assertEq(tokenA.balanceOf(address(this)), 1_000_000 ether - 500 ether, "tokenA spent");
        assertEq(tokenC.balanceOf(address(this)), 600 ether, "tokenC received");
        assertEq(tokenB.balanceOf(address(router)), 0, "no residual tokenB");
        // Cannot assert finalDelta directly as execute returns void, but balances confirm correctness.
        assertEq(pool.settleHistoryLength(), 2, "two settle calls");
        assertEq(pool.takeHistoryLength(), 2, "two take calls");
    }

    function testMultiHopRevertsOnFinalSlippage() external {
        (
            BaygusRouter router,
            MockPoolManager pool,
            MockERC20 tokenA,
            MockERC20 tokenB,
            MockERC20 tokenC
        ) = _deployEnvironment();

        pool.queueDelta(int128(int256(800 ether)), int128(int256(-500 ether)));
        pool.queueDelta(int128(int256(500 ether)), int128(int256(-800 ether)));

        tokenA.approve(address(router), type(uint256).max);

        BaygusRouter.Hop[] memory hops = new BaygusRouter.Hop[](2);
        hops[0] = BaygusRouter.Hop({
            key: _poolKey(address(tokenB), address(tokenA)),
            params: _params(),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        hops[1] = BaygusRouter.Hop({
            key: _poolKey(address(tokenC), address(tokenB)),
            params: _params(),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });

        bytes memory innerPayload = abi.encode(
            address(this),
            address(this),
            int128(int256(600 ether)),
            int128(0),
            hops
        );
        bytes memory input = abi.encode(uint8(1), innerPayload);
        
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = input;
        bytes memory commands = abi.encodePacked(uint8(CMD_V4_SWAP));

        bytes memory callData = abi.encodeWithSelector(
            BaygusRouter.execute.selector,
            commands,
            inputs
        );

        (bool success, bytes memory returndata) = address(router).call(callData);
        assertTrue(!success, "expected final slippage revert");
        returndata;
    }
}
