// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {PoolKey, SwapParams, BalanceDelta} from "../src/types/SharedTypes.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockHookAdapter} from "./mocks/MockHookAdapter.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusRouterTest is TestBase {
    function _deployEnvironment()
        internal
        returns (BaygusRouter router, MockPoolManager pool, MockERC20 token, MockERC20 weth)
    {
        token = new MockERC20("Token", "TKN", 18);
        weth = new MockERC20("Wrapped ETH", "WETH", 18);

        pool = new MockPoolManager();
        router = new BaygusRouter(address(pool));
        pool.setRouter(address(router));

        token.mint(address(pool), 1_000_000 ether);
        weth.mint(address(pool), 1_000_000 ether);
    }

    function _poolKey(address token, address weth) internal pure returns (PoolKey memory) {
        return PoolKey({currency0: token, currency1: weth, fee: 1_000, tickSpacing: 1});
    }

    function _params(bool zeroForOne) internal pure returns (SwapParams memory) {
        return SwapParams({zeroForOne: zeroForOne, amountSpecified: 1 ether, sqrtPriceLimitX96: 0});
    }

    function _request(
        PoolKey memory key,
        SwapParams memory params,
        address recipient
    ) internal pure returns (BaygusRouter.SwapExactInputSingleParams memory) {
        return BaygusRouter.SwapExactInputSingleParams({
            key: key,
            params: params,
            recipient: recipient,
            hookData: "",
            hookAdapter: address(0),
            minAmount0: 0,
            minAmount1: 0
        });
    }

    function testSwapExactInputSingle_BuysToken() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 token, MockERC20 weth) =
            _deployEnvironment();

        pool.setNextDelta(int128(int256(1500 ether)), int128(int256(-500 ether)));

        weth.mint(address(this), 500 ether);
        weth.approve(address(router), 500 ether);

        uint256 preToken = token.balanceOf(address(this));
        uint256 preWethTrader = weth.balanceOf(address(this));
        uint256 preWethPool = weth.balanceOf(address(pool));

        BaygusRouter.SwapExactInputSingleParams memory req = _request(
            _poolKey(address(token), address(weth)),
            _params(false),
            address(this)
        );

        BalanceDelta memory delta = router.swapExactInputSingle(req);

        assertEq(token.balanceOf(address(this)), preToken + 1500 ether, "token out mismatch");
        assertEq(weth.balanceOf(address(this)), preWethTrader - 500 ether, "weth debit mismatch");
        assertEq(weth.balanceOf(address(pool)), preWethPool + 500 ether, "pool WETH increment");

        assertEq(pool.settleHistoryLength(), 1, "settle history length");
        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(weth), "settle currency");
        assertEq(settleCall.amount, 500 ether, "settle amount");
        assertTrue(!settleCall.isNative, "settle should be ERC20");

        assertEq(pool.takeHistoryLength(), 1, "take history length");
        MockPoolManager.TakeCall memory takeCall = pool.getTakeCall(0);
        assertEq(takeCall.currency, address(token), "take currency");
        assertEq(takeCall.recipient, address(this), "take recipient");
        assertEq(takeCall.amount, 1500 ether, "take amount");

        assertEq(uint256(int256(delta.amount0)), 1500 ether, "delta amount0");
        assertEq(uint256(int256(-delta.amount1)), 500 ether, "delta amount1");
    }

    function testSwapExactInputSingle_SellsToken() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 token, MockERC20 weth) =
            _deployEnvironment();

        pool.setNextDelta(int128(int256(-250 ether)), int128(int256(200 ether)));

        token.mint(address(this), 250 ether);
        token.approve(address(router), 250 ether);

        uint256 preTokenTrader = token.balanceOf(address(this));
        uint256 preTokenPool = token.balanceOf(address(pool));
        uint256 preWethTrader = weth.balanceOf(address(this));
        uint256 preWethPool = weth.balanceOf(address(pool));

        BaygusRouter.SwapExactInputSingleParams memory req = _request(
            _poolKey(address(token), address(weth)),
            _params(true),
            address(this)
        );

        BalanceDelta memory delta = router.swapExactInputSingle(req);

        assertEq(token.balanceOf(address(this)), preTokenTrader - 250 ether, "token debit mismatch");
        assertEq(token.balanceOf(address(pool)), preTokenPool + 250 ether, "pool token credit");
        assertEq(weth.balanceOf(address(this)), preWethTrader + 200 ether, "weth credit mismatch");
        assertEq(weth.balanceOf(address(pool)), preWethPool - 200 ether, "pool weth debit");

        assertEq(pool.settleHistoryLength(), 1, "settle history length sell");
        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(token), "settle currency sell");
        assertEq(settleCall.amount, 250 ether, "settle amount sell");
        assertTrue(!settleCall.isNative, "settle should be ERC20 sell");

        assertEq(pool.takeHistoryLength(), 1, "take history length sell");
        MockPoolManager.TakeCall memory takeCall = pool.getTakeCall(0);
        assertEq(takeCall.currency, address(weth), "take currency sell");
        assertEq(takeCall.recipient, address(this), "take recipient sell");
        assertEq(takeCall.amount, 200 ether, "take amount sell");

        assertEq(uint256(int256(-delta.amount0)), 250 ether, "delta amount0 sell");
        assertEq(uint256(int256(delta.amount1)), 200 ether, "delta amount1 sell");
    }

    function testSwapExactInputSingle_WithHookAdapter() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 token, MockERC20 weth) =
            _deployEnvironment();

        pool.setNextDelta(int128(int256(100 ether)), int128(int256(-40 ether)));

        weth.mint(address(this), 40 ether);
        weth.approve(address(router), 40 ether);

        MockHookAdapter hook = new MockHookAdapter();
        bytes memory hookData = bytes("custom-hook-data");

        BaygusRouter.SwapExactInputSingleParams memory req = _request(
            _poolKey(address(token), address(weth)),
            _params(false),
            address(this)
        );
        req.hookData = hookData;
        req.hookAdapter = address(hook);

        router.swapExactInputSingle(req);

        assertTrue(hook.beforeCalled(), "before hook not invoked");
        assertTrue(hook.afterCalled(), "after hook not invoked");
        assertEq(hook.afterAmount0(), int128(int256(100 ether)), "hook amount0 mismatch");
        assertEq(hook.afterAmount1(), int128(int256(-40 ether)), "hook amount1 mismatch");
        assertEq(hook.beforeSender(), address(this), "before sender");
        assertEq(hook.afterRecipient(), address(this), "after recipient");
        assertEq(hook.lastHookData(), hookData, "hook data");
    }

    function testSwapExactInputSingle_RevertsOnSlippage() external {
        (BaygusRouter router, MockPoolManager pool, MockERC20 token, MockERC20 weth) =
            _deployEnvironment();

        pool.setNextDelta(int128(int256(90 ether)), int128(int256(-30 ether)));

        weth.mint(address(this), 30 ether);
        weth.approve(address(router), 30 ether);

        BaygusRouter.SwapExactInputSingleParams memory req = _request(
            _poolKey(address(token), address(weth)),
            _params(false),
            address(this)
        );
        req.minAmount0 = int128(int256(100 ether));

        bytes memory callData = abi.encodeWithSelector(
            BaygusRouter.swapExactInputSingle.selector,
            req
        );
        (bool success, ) = address(router).call(callData);
        assertTrue(!success, "expected slippage revert");
    }
}
