// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ILockCallback} from "./interfaces/ILockCallback.sol";
import {IPoolManager} from "./interfaces/IPoolManager.sol";
import {IERC20} from "./interfaces/IERC20.sol";
import {IHookAdapter} from "./interfaces/IHookAdapter.sol";
import {PoolKey, SwapParams, BalanceDelta} from "./types/SharedTypes.sol";

/// @title BaygusRouter
/// @notice Router used by the Baygus execution agent: performs Uniswap v4 PoolManager lock →
///         swap → settle flows with optional hook adapters and per-leg slippage checks.
/// @dev Designed for simulation and staged deployment; future versions will layer in multi-hop
///      support and additional venue adapters.
contract BaygusRouter is ILockCallback {
    address public immutable poolManager;

    bool private _entered;
    uint256 private _nativeBuffer;

    struct SwapExactInputSingleParams {
        PoolKey key;
        SwapParams params;
        address recipient;
        bytes hookData;
        address hookAdapter;
        int128 minAmount0;
        int128 minAmount1;
    }

    struct Hop {
        PoolKey key;
        SwapParams params;
        bytes hookData;
        address hookAdapter;
        int128 minAmount0;
        int128 minAmount1;
    }

    struct MultiHopParams {
        Hop[] hops;
        address recipient;
        int128 finalMinAmount0;
        int128 finalMinAmount1;
    }

    struct MultiHopContext {
        address initiator;
        address recipient;
        int128 finalMinAmount0;
        int128 finalMinAmount1;
        Hop[] hops;
    }

    struct SwapContext {
        address sender;
        address recipient;
        PoolKey key;
        SwapParams params;
        bytes hookData;
        address hookAdapter;
        int128 minAmount0;
        int128 minAmount1;
    }

    error EmptyPath();
    error RouterReentrant();
    error InvalidRecipient();
    error MissingPoolManager();
    error InsufficientNativeLiquidity();
    error ERC20TransferFailed();
    error NativeNotFullyConsumed();
    error UnauthorizedPoolManager();
    error SlippageCheckFailed(uint8 index, int128 actual, int128 minimum);

    constructor(address poolManager_) {
        if (poolManager_ == address(0)) revert MissingPoolManager();
        poolManager = poolManager_;
    }

    function swapExactInputSingle(
        SwapExactInputSingleParams calldata request
    ) external payable returns (BalanceDelta memory delta) {
        if (_entered) revert RouterReentrant();
        if (request.recipient == address(0)) revert InvalidRecipient();

        _entered = true;
        uint256 previousNative = _nativeBuffer;
        _nativeBuffer = msg.value;

        bytes memory payload = abi.encode(uint8(0), abi.encode(request, msg.sender));
        bytes memory response = IPoolManager(poolManager).lock(payload);
        delta = abi.decode(response, (BalanceDelta));

        if (_nativeBuffer != 0) revert NativeNotFullyConsumed();
        _nativeBuffer = previousNative;
        _entered = false;

        return delta;
    }

    function swapExactInputPath(
        MultiHopParams calldata request
    ) external payable returns (BalanceDelta memory finalDelta) {
        if (request.hops.length == 0) revert EmptyPath();
        if (_entered) revert RouterReentrant();
        if (request.recipient == address(0)) revert InvalidRecipient();

        _entered = true;
        uint256 previousNative = _nativeBuffer;
        _nativeBuffer = msg.value;

        bytes memory payload = abi.encode(
            uint8(1),
            abi.encode(
                msg.sender,
                request.recipient,
                request.finalMinAmount0,
                request.finalMinAmount1,
                request.hops
            )
        );

        bytes memory response = IPoolManager(poolManager).lock(payload);
        finalDelta = abi.decode(response, (BalanceDelta));

        if (_nativeBuffer != 0) revert NativeNotFullyConsumed();
        _nativeBuffer = previousNative;
        _entered = false;

        return finalDelta;
    }

    function lockAcquired(bytes calldata data) external override returns (bytes memory) {
        if (msg.sender != poolManager) revert UnauthorizedPoolManager();

        (uint8 op, bytes memory payload) = abi.decode(data, (uint8, bytes));
        if (op == 0) {
            (SwapExactInputSingleParams memory request, address singleInitiator) = abi.decode(
                payload,
                (SwapExactInputSingleParams, address)
            );
            BalanceDelta memory delta = _executeSingle(request, singleInitiator);
            return abi.encode(delta);
        }

        (
            address initiator,
            address recipient,
            int128 finalMinAmount0,
            int128 finalMinAmount1,
            Hop[] memory hops
        ) = abi.decode(payload, (address, address, int128, int128, Hop[]));

        BalanceDelta memory finalDelta = _executeMultiHop(
            initiator,
            recipient,
            finalMinAmount0,
            finalMinAmount1,
            hops
        );
        return abi.encode(finalDelta);
    }

    function _executeSingle(
        SwapExactInputSingleParams memory request,
        address initiator
    ) internal returns (BalanceDelta memory delta) {
        SwapContext memory context = SwapContext({
            sender: initiator,
            recipient: request.recipient,
            key: request.key,
            params: request.params,
            hookData: request.hookData,
            hookAdapter: request.hookAdapter,
            minAmount0: request.minAmount0,
            minAmount1: request.minAmount1
        });

        _invokeBeforeSwap(context);
        delta = IPoolManager(poolManager).swap(context.key, context.params, context.hookData);
        _handleSettlement(context, delta);
        _validateDelta(context, delta);
        _invokeAfterSwap(context, delta);
        return delta;
    }

    function _executeMultiHop(
        address initiator,
        address finalRecipient,
        int128 finalMinAmount0,
        int128 finalMinAmount1,
        Hop[] memory hops
    ) internal returns (BalanceDelta memory finalDelta) {
        address currentSender = initiator;
        for (uint256 i = 0; i < hops.length; ++i) {
            Hop memory hop = hops[i];

            SwapContext memory context = SwapContext({
                sender: currentSender,
                recipient: i + 1 == hops.length ? finalRecipient : address(this),
                key: hop.key,
                params: hop.params,
                hookData: hop.hookData,
                hookAdapter: hop.hookAdapter,
                minAmount0: hop.minAmount0,
                minAmount1: hop.minAmount1
            });

            _invokeBeforeSwap(context);
            BalanceDelta memory delta = IPoolManager(poolManager).swap(
                context.key,
                context.params,
                context.hookData
            );
            _handleSettlement(context, delta);
            _validateDelta(context, delta);
            _invokeAfterSwap(context, delta);

            currentSender = address(this);
            finalDelta = delta;
        }

        if (finalMinAmount0 != 0 && finalDelta.amount0 < finalMinAmount0) {
            revert SlippageCheckFailed(2, finalDelta.amount0, finalMinAmount0);
        }
        if (finalMinAmount1 != 0 && finalDelta.amount1 < finalMinAmount1) {
            revert SlippageCheckFailed(3, finalDelta.amount1, finalMinAmount1);
        }

        return finalDelta;
    }

    function _handleSettlement(SwapContext memory context, BalanceDelta memory delta) internal {
        if (delta.amount0 < 0) {
            _settleCurrency(context.sender, context.key.currency0, _abs(delta.amount0));
        } else if (delta.amount0 > 0) {
            _takeCurrency(context.recipient, context.key.currency0, _abs(delta.amount0));
        }

        if (delta.amount1 < 0) {
            _settleCurrency(context.sender, context.key.currency1, _abs(delta.amount1));
        } else if (delta.amount1 > 0) {
            _takeCurrency(context.recipient, context.key.currency1, _abs(delta.amount1));
        }
    }

    function _settleCurrency(address payer, address currency, uint256 amount) internal {
        if (amount == 0) return;

        if (currency == address(0)) {
            if (_nativeBuffer < amount) revert InsufficientNativeLiquidity();
            _nativeBuffer -= amount;
            IPoolManager(poolManager).settle{value: amount}(currency, amount);
        } else {
            bool ok;
            if (payer == address(this)) {
                ok = IERC20(currency).transfer(poolManager, amount);
            } else {
                ok = IERC20(currency).transferFrom(payer, poolManager, amount);
            }
            if (!ok) revert ERC20TransferFailed();
            IPoolManager(poolManager).settle(currency, amount);
        }
    }

    function _takeCurrency(address recipient, address currency, uint256 amount) internal {
        if (amount == 0) return;
        IPoolManager(poolManager).take(currency, recipient, amount);
    }

    function _invokeBeforeSwap(SwapContext memory context) internal {
        if (context.hookAdapter == address(0)) return;
        IHookAdapter(context.hookAdapter).beforeSwap(
            context.sender,
            context.recipient,
            context.key,
            context.params,
            context.hookData
        );
    }

    function _invokeAfterSwap(SwapContext memory context, BalanceDelta memory delta) internal {
        if (context.hookAdapter == address(0)) return;
        IHookAdapter(context.hookAdapter).afterSwap(
            context.sender,
            context.recipient,
            context.key,
            context.params,
            delta.amount0,
            delta.amount1,
            context.hookData
        );
    }

    function _validateDelta(SwapContext memory context, BalanceDelta memory delta) internal pure {
        if (context.minAmount0 != 0 && delta.amount0 < context.minAmount0) {
            revert SlippageCheckFailed(0, delta.amount0, context.minAmount0);
        }
        if (context.minAmount1 != 0 && delta.amount1 < context.minAmount1) {
            revert SlippageCheckFailed(1, delta.amount1, context.minAmount1);
        }
    }

    function _abs(int128 value) internal pure returns (uint256) {
        return uint256(int256(value < 0 ? -value : value));
    }

    receive() external payable {}
}
