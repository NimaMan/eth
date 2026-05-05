// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {AdapterConfig, PoolKey, SwapParams, BalanceDelta} from "../src/types/SharedTypes.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockHookAdapter} from "./mocks/MockHookAdapter.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusRouterV4Test is TestBase {
    function _deploy()
        internal
        returns (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB, MockERC20 tokenC)
    {
        tokenA = new MockERC20("Token A", "TKNA", 18);
        tokenB = new MockERC20("Token B", "TKNB", 18);
        tokenC = new MockERC20("Token C", "TKNC", 18);
        pool = new MockPoolManager();

        AdapterConfig memory adapters = AdapterConfig({
            uniswapV2Router: address(0),
            sushiswapRouter: address(0),
            uniswapV3Router: address(0),
            balancerVault: address(0),
            permit2: address(0)
        });
        router = new BaygusRouter(address(pool), adapters);
        pool.setRouter(address(router));

        tokenA.mint(address(this), 1_000_000 ether);
        tokenA.mint(address(pool), 1_000_000 ether);
        tokenB.mint(address(pool), 1_000_000 ether);
        tokenC.mint(address(pool), 1_000_000 ether);
        pool.sync(address(tokenA));
        pool.sync(address(tokenB));
        pool.sync(address(tokenC));
    }

    function _key(address currency0, address currency1) internal pure returns (PoolKey memory) {
        return PoolKey({currency0: currency0, currency1: currency1, fee: 1_000, tickSpacing: 1, hooks: address(0)});
    }

    function _params(bool zeroForOne) internal pure returns (SwapParams memory) {
        return SwapParams({zeroForOne: zeroForOne, amountSpecified: -1 ether, sqrtPriceLimitX96: 0});
    }

    function testSwapExactInputSingleSettlesAndTakes() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB,) = _deploy();

        pool.setNextDelta(int128(int256(150 ether)), int128(int256(-50 ether)));
        tokenB.mint(address(this), 50 ether);
        tokenB.approve(address(router), 50 ether);
        pool.sync(address(tokenB));

        BaygusRouter.SwapExactInputSingleParams memory request = BaygusRouter.SwapExactInputSingleParams({
            key: _key(address(tokenA), address(tokenB)),
            params: _params(false),
            recipient: address(this),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: int128(int256(150 ether)),
            minAmount1: 0
        });

        BalanceDelta memory delta = router.swapExactInputSingle(request);

        assertEq(delta.amount0, int128(int256(150 ether)), "delta token out");
        assertEq(delta.amount1, int128(int256(-50 ether)), "delta input");
        assertEq(tokenA.balanceOf(address(this)), 1_000_000 ether + 150 ether, "token output");
        assertEq(tokenB.balanceOf(address(this)), 0, "input spent");
        assertEq(pool.settleHistoryLength(), 1, "one settle");
        assertEq(pool.takeHistoryLength(), 1, "one take");

        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(tokenB), "settle currency");
        assertEq(settleCall.amount, 50 ether, "settle amount");

        MockPoolManager.TakeCall memory takeCall = pool.getTakeCall(0);
        assertEq(takeCall.currency, address(tokenA), "take currency");
        assertEq(takeCall.recipient, address(this), "take recipient");
        assertEq(takeCall.amount, 150 ether, "take amount");
    }

    function testSwapExactInputSingleSettlesNativeAndTakesToken() external {
        (BaygusRouter router, MockPoolManager pool,, MockERC20 tokenB,) = _deploy();
        vm.deal(address(this), 100 ether);

        pool.setNextDelta(int128(int256(-50 ether)), int128(int256(10 ether)));

        BaygusRouter.SwapExactInputSingleParams memory request = BaygusRouter.SwapExactInputSingleParams({
            key: _key(address(0), address(tokenB)),
            params: _params(true),
            recipient: address(this),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: int128(int256(10 ether))
        });

        BalanceDelta memory delta = router.swapExactInputSingle{value: 50 ether}(request);

        assertEq(delta.amount0, int128(int256(-50 ether)), "native input delta");
        assertEq(delta.amount1, int128(int256(10 ether)), "token output delta");
        assertEq(tokenB.balanceOf(address(this)), 10 ether, "token output");
        assertEq(pool.settleHistoryLength(), 1, "one native settle");
        assertEq(pool.takeHistoryLength(), 1, "one token take");

        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(0), "settle native");
        assertEq(settleCall.amount, 50 ether, "settle native amount");
        assertTrue(settleCall.isNative, "native settle flag");
    }

    function testSwapExactInputSingleRevertsOnSlippage() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB,) = _deploy();

        pool.setNextDelta(int128(int256(100 ether)), int128(int256(-50 ether)));
        tokenB.mint(address(this), 50 ether);
        tokenB.approve(address(router), 50 ether);
        pool.sync(address(tokenB));

        BaygusRouter.SwapExactInputSingleParams memory request = BaygusRouter.SwapExactInputSingleParams({
            key: _key(address(tokenA), address(tokenB)),
            params: _params(false),
            recipient: address(this),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: int128(int256(150 ether)),
            minAmount1: 0
        });

        (bool success,) = address(router).call(abi.encodeCall(BaygusRouter.swapExactInputSingle, (request)));
        assertFalse(success, "expected single-hop slippage failure");
    }

    function testSwapExactInputSingleInvokesHook() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB,) = _deploy();

        pool.setNextDelta(int128(int256(10 ether)), int128(int256(-5 ether)));
        tokenB.mint(address(this), 5 ether);
        tokenB.approve(address(router), 5 ether);
        pool.sync(address(tokenB));

        MockHookAdapter hook = new MockHookAdapter();
        bytes memory hookData = "hook-context";
        BaygusRouter.SwapExactInputSingleParams memory request = BaygusRouter.SwapExactInputSingleParams({
            key: _key(address(tokenA), address(tokenB)),
            params: _params(false),
            recipient: address(this),
            hookData: hookData,
            hookAdapter: address(hook),
            minAmount0: 0,
            minAmount1: 0
        });

        router.swapExactInputSingle(request);

        assertTrue(hook.beforeCalled(), "before hook");
        assertTrue(hook.afterCalled(), "after hook");
        assertEq(hook.beforeSender(), address(this), "hook sender");
        assertEq(hook.afterRecipient(), address(this), "hook recipient");
        assertEq(hook.afterAmount0(), int128(int256(10 ether)), "hook amount0");
        assertEq(hook.afterAmount1(), int128(int256(-5 ether)), "hook amount1");
        assertEq(hook.lastHookData(), hookData, "hook data");
    }

    function testSwapExactInputPathNetsIntermediateCurrency() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB, MockERC20 tokenC) = _deploy();

        pool.queueDelta(int128(int256(800 ether)), int128(int256(-500 ether)));
        pool.queueDelta(int128(int256(600 ether)), int128(int256(-800 ether)));
        tokenA.approve(address(router), type(uint256).max);

        BaygusRouter.Hop[] memory hops = new BaygusRouter.Hop[](2);
        hops[0] = BaygusRouter.Hop({
            key: _key(address(tokenB), address(tokenA)),
            params: _params(false),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        hops[1] = BaygusRouter.Hop({
            key: _key(address(tokenC), address(tokenB)),
            params: _params(false),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        BaygusRouter.MultiHopParams memory route = BaygusRouter.MultiHopParams({
            hops: hops, recipient: address(this), finalMinAmount0: int128(int256(600 ether)), finalMinAmount1: 0
        });

        router.swapExactInputPath(route);

        assertEq(tokenA.balanceOf(address(this)), 1_000_000 ether - 500 ether, "input spent");
        assertEq(tokenB.balanceOf(address(router)), 0, "no intermediate residue");
        assertEq(tokenC.balanceOf(address(this)), 600 ether, "final output");
        assertEq(pool.settleHistoryLength(), 1, "one net settle");
        assertEq(pool.takeHistoryLength(), 1, "one net take");

        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(tokenA), "settle input");
        assertEq(settleCall.amount, 500 ether, "settle amount");

        MockPoolManager.TakeCall memory takeCall = pool.getTakeCall(0);
        assertEq(takeCall.currency, address(tokenC), "take final");
        assertEq(takeCall.amount, 600 ether, "take amount");
    }

    function testSwapExactInputPathRevertsOnFinalSlippage() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB, MockERC20 tokenC) = _deploy();

        pool.queueDelta(int128(int256(800 ether)), int128(int256(-500 ether)));
        pool.queueDelta(int128(int256(500 ether)), int128(int256(-800 ether)));
        tokenA.approve(address(router), type(uint256).max);

        BaygusRouter.Hop[] memory hops = new BaygusRouter.Hop[](2);
        hops[0] = BaygusRouter.Hop({
            key: _key(address(tokenB), address(tokenA)),
            params: _params(false),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        hops[1] = BaygusRouter.Hop({
            key: _key(address(tokenC), address(tokenB)),
            params: _params(false),
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
        BaygusRouter.MultiHopParams memory route = BaygusRouter.MultiHopParams({
            hops: hops, recipient: address(this), finalMinAmount0: int128(int256(600 ether)), finalMinAmount1: 0
        });

        (bool success,) = address(router).call(abi.encodeCall(BaygusRouter.swapExactInputPath, (route)));
        assertFalse(success, "expected slippage failure");
    }

    function testSwapExactInputPathRejectsEmptyPath() external {
        (BaygusRouter router,,,,) = _deploy();

        BaygusRouter.Hop[] memory hops = new BaygusRouter.Hop[](0);
        BaygusRouter.MultiHopParams memory route =
            BaygusRouter.MultiHopParams({hops: hops, recipient: address(this), finalMinAmount0: 0, finalMinAmount1: 0});

        (bool success,) = address(router).call(abi.encodeCall(BaygusRouter.swapExactInputPath, (route)));
        assertFalse(success, "expected empty path failure");
    }

    function testUnlockCallbackRejectsUnauthorizedCaller() external {
        (BaygusRouter router,,,,) = _deploy();

        (bool success,) = address(router).call(abi.encodeCall(BaygusRouter.unlockCallback, (abi.encode(uint8(0), ""))));
        assertFalse(success, "expected unauthorized callback failure");
    }
}
