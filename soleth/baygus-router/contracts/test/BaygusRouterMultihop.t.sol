// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {PoolKey, SwapParams, BalanceDelta} from "../src/types/SharedTypes.sol";
import {ILockCallback} from "../src/interfaces/ILockCallback.sol";
import {IPoolManager} from "../src/interfaces/IPoolManager.sol";
import {IERC20} from "../src/interfaces/IERC20.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {TestBase} from "./utils/TestBase.sol";

/// @dev Test-only harness that preserves the legacy per-hop settle/take behaviour
///      so we can benchmark gas against the new aggregated path.
contract LegacyMultihopRouter is ILockCallback {
    address public immutable poolManager;

    error UnauthorizedPoolManager();
    error SlippageCheckFailed(uint8 index, int128 actual, int128 minimum);

    constructor(address poolManager_) {
        poolManager = poolManager_;
    }

    function execute(bytes calldata commands, bytes[] calldata inputs) external payable {
        for (uint256 i = 0; i < commands.length; ) {
            IPoolManager(poolManager).unlock(inputs[i]);
            unchecked {
                ++i;
            }
        }
    }

    function unlockCallback(bytes calldata data) external override returns (bytes memory) {
        if (msg.sender != poolManager) revert UnauthorizedPoolManager();

        (uint8 op, bytes memory payload) = abi.decode(data, (uint8, bytes));
        if (op != 1) revert UnauthorizedPoolManager();

        (
            address initiator,
            address recipient,
            int128 finalMinAmount0,
            int128 finalMinAmount1,
            BaygusRouter.Hop[] memory hops
        ) = abi.decode(payload, (address, address, int128, int128, BaygusRouter.Hop[]));

        BalanceDelta memory delta = _executeMultiHopLegacy(
            initiator,
            recipient,
            finalMinAmount0,
            finalMinAmount1,
            hops
        );
        return abi.encode(delta);
    }

    function _executeMultiHopLegacy(
        address initiator,
        address finalRecipient,
        int128 finalMinAmount0,
        int128 finalMinAmount1,
        BaygusRouter.Hop[] memory hops
    ) internal returns (BalanceDelta memory finalDelta) {
        address currentSender = initiator;
        for (uint256 i = 0; i < hops.length; ) {
            BaygusRouter.Hop memory hop = hops[i];
            address recipient = i + 1 == hops.length ? finalRecipient : address(this);

            BalanceDelta memory delta = IPoolManager(poolManager).swap(hop.key, hop.params, hop.hookData);

            _handleSettlement(currentSender, recipient, hop.key, delta);
            _validateDelta(hop, delta);

            currentSender = address(this);
            finalDelta = delta;
            unchecked {
                ++i;
            }
        }

        if (finalMinAmount0 != 0 && finalDelta.amount0 > finalMinAmount0) {
            revert SlippageCheckFailed(2, finalDelta.amount0, finalMinAmount0);
        }
        if (finalMinAmount1 != 0 && finalDelta.amount1 > finalMinAmount1) {
            revert SlippageCheckFailed(3, finalDelta.amount1, finalMinAmount1);
        }

        return finalDelta;
    }

    function _handleSettlement(
        address sender,
        address recipient,
        PoolKey memory key,
        BalanceDelta memory delta
    ) internal {
        if (delta.amount0 > 0) {
            _settleCurrency(sender, key.currency0, uint256(int256(delta.amount0)));
        } else if (delta.amount0 < 0) {
            _takeCurrency(recipient, key.currency0, uint256(int256(-delta.amount0)));
        }

        if (delta.amount1 > 0) {
            _settleCurrency(sender, key.currency1, uint256(int256(delta.amount1)));
        } else if (delta.amount1 < 0) {
            _takeCurrency(recipient, key.currency1, uint256(int256(-delta.amount1)));
        }
    }

    function _settleCurrency(address payer, address currency, uint256 amount) internal {
        if (amount == 0) return;

        if (currency == address(0)) revert UnauthorizedPoolManager();

        bool ok;
        if (payer == address(this)) {
            ok = IERC20(currency).transfer(poolManager, amount);
        } else {
            ok = IERC20(currency).transferFrom(payer, poolManager, amount);
        }
        if (!ok) revert UnauthorizedPoolManager();
        IPoolManager(poolManager).settle(currency);
    }

    function _takeCurrency(address recipient, address currency, uint256 amount) internal {
        if (amount == 0) return;
        IPoolManager(poolManager).take(currency, recipient, amount);
    }

    function _validateDelta(BaygusRouter.Hop memory hop, BalanceDelta memory delta) internal pure {
        if (hop.minAmount0 != 0 && delta.amount0 > hop.minAmount0) {
            revert SlippageCheckFailed(0, delta.amount0, hop.minAmount0);
        }
        if (hop.minAmount1 != 0 && delta.amount1 > hop.minAmount1) {
            revert SlippageCheckFailed(1, delta.amount1, hop.minAmount1);
        }
    }
}

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

    function _deployLegacyEnvironment()
        internal
        returns (LegacyMultihopRouter router, MockPoolManager pool, MockERC20 tokenA, MockERC20 tokenB, MockERC20 tokenC)
    {
        tokenA = new MockERC20("TokenA", "TKNA", 18);
        tokenB = new MockERC20("TokenB", "TKNB", 18);
        tokenC = new MockERC20("TokenC", "TKNC", 18);

        pool = new MockPoolManager();
        router = new LegacyMultihopRouter(address(pool));
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

    function _buildStandardHops(address tokenA, address tokenB, address tokenC)
        internal
        pure
        returns (BaygusRouter.Hop[] memory hops)
    {
        hops = new BaygusRouter.Hop[](2);
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
    }

    function _buildMultiHopCalldata(
        address initiator,
        address recipient,
        int128 finalMinAmount0,
        int128 finalMinAmount1,
        BaygusRouter.Hop[] memory hops
    ) internal pure returns (bytes memory commands, bytes[] memory inputs) {
        bytes memory innerPayload = abi.encode(
            initiator,
            recipient,
            finalMinAmount0,
            finalMinAmount1,
            hops
        );
        bytes memory input = abi.encode(uint8(1), innerPayload);
        inputs = new bytes[](1);
        inputs[0] = input;
        commands = abi.encodePacked(uint8(CMD_V4_SWAP));
    }

    function testMultiHopExactInputPath() external {
        (
            BaygusRouter router,
            MockPoolManager pool,
            MockERC20 tokenA,
            MockERC20 tokenB,
            MockERC20 tokenC
        ) = _deployEnvironment();

        // Hop 1: A -> B.
        // PoolKey(B, A).
        // Pay 500 A (+), Receive 800 B (-).
        pool.queueDelta(int128(int256(-800 ether)), int128(int256(500 ether)));
        
        // Hop 2: B -> C.
        // PoolKey(C, B).
        // Pay 800 B (+), Receive 600 C (-).
        pool.queueDelta(int128(int256(-600 ether)), int128(int256(800 ether)));

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

        // Final amounts are checked against Hop 2 output (C).
        // We expect 600 C.
        bytes memory innerPayload = abi.encode(
            address(this),
            address(this),
            int128(int256(-600 ether)), // minAmount0 (C)
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
        assertEq(pool.settleHistoryLength(), 1, "aggregated settle call");
        assertEq(pool.takeHistoryLength(), 1, "aggregated take call");
        MockPoolManager.SettleCall memory settleCall = pool.getSettleCall(0);
        assertEq(settleCall.currency, address(tokenA), "settle currency is input token");
        assertEq(settleCall.amount, 500 ether, "settle amount aggregates hop debt");
        MockPoolManager.TakeCall memory takeCall = pool.getTakeCall(0);
        assertEq(takeCall.currency, address(tokenC), "take currency is final output");
        assertEq(takeCall.amount, 600 ether, "take amount aggregates hop credit");
    }

    function testMultiHopRevertsOnFinalSlippage() external {
        (
            BaygusRouter router,
            MockPoolManager pool,
            MockERC20 tokenA,
            MockERC20 tokenB,
            MockERC20 tokenC
        ) = _deployEnvironment();

        // Hop 1: Pay 500 A, Receive 800 B.
        pool.queueDelta(int128(int256(-800 ether)), int128(int256(500 ether)));
        
        // Hop 2: Pay 800 B, Receive 500 C.
        pool.queueDelta(int128(int256(-500 ether)), int128(int256(800 ether)));

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

        // We expect at least 600 C (-600).
        // We receive 500 C (-500).
        // -500 > -600. Revert.
        bytes memory innerPayload = abi.encode(
            address(this),
            address(this),
            int128(int256(-600 ether)),
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

    function testMultiHopGas_NetSettlementCheaperThanLegacy() external {
        (
            BaygusRouter router,
            MockPoolManager pool,
            MockERC20 tokenA,
            MockERC20 tokenB,
            MockERC20 tokenC
        ) = _deployEnvironment();

        BaygusRouter.Hop[] memory hops = _buildStandardHops(address(tokenA), address(tokenB), address(tokenC));

        pool.queueDelta(int128(int256(-800 ether)), int128(int256(500 ether)));
        pool.queueDelta(int128(int256(-600 ether)), int128(int256(800 ether)));
        tokenA.approve(address(router), type(uint256).max);
        (bytes memory commands, bytes[] memory inputs) = _buildMultiHopCalldata(
            address(this),
            address(this),
            int128(int256(-600 ether)),
            int128(0),
            hops
        );

        uint256 startGas = gasleft();
        router.execute(commands, inputs);
        uint256 aggregatedGas = startGas - gasleft();

        (
            LegacyMultihopRouter legacyRouter,
            MockPoolManager legacyPool,
            MockERC20 legacyTokenA,
            MockERC20 legacyTokenB,
            MockERC20 legacyTokenC
        ) = _deployLegacyEnvironment();

        BaygusRouter.Hop[] memory legacyHops =
            _buildStandardHops(address(legacyTokenA), address(legacyTokenB), address(legacyTokenC));

        legacyPool.queueDelta(int128(int256(-800 ether)), int128(int256(500 ether)));
        legacyPool.queueDelta(int128(int256(-600 ether)), int128(int256(800 ether)));
        legacyTokenA.approve(address(legacyRouter), type(uint256).max);
        (commands, inputs) = _buildMultiHopCalldata(
            address(this),
            address(this),
            int128(int256(-600 ether)),
            int128(0),
            legacyHops
        );

        startGas = gasleft();
        legacyRouter.execute(commands, inputs);
        uint256 legacyGas = startGas - gasleft();

        assertTrue(aggregatedGas < legacyGas, "aggregated settlement should reduce gas");
    }
}
