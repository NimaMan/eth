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

        SwapContext memory context = SwapContext({
            sender: msg.sender,
            recipient: request.recipient,
            key: request.key,
            params: request.params,
            hookData: request.hookData,
            hookAdapter: request.hookAdapter,
            minAmount0: request.minAmount0,
            minAmount1: request.minAmount1
        });

        _invokeBeforeSwap(context);

        bytes memory response = IPoolManager(poolManager).lock(abi.encode(context));
        delta = abi.decode(response, (BalanceDelta));

        if (_nativeBuffer != 0) revert NativeNotFullyConsumed();
        _nativeBuffer = previousNative;
        _entered = false;

        _validateDelta(context, delta);
        _invokeAfterSwap(context, delta);

        return delta;
    }

    function lockAcquired(bytes calldata data) external override returns (bytes memory) {
        if (msg.sender != poolManager) revert UnauthorizedPoolManager();

        SwapContext memory context = abi.decode(data, (SwapContext));
        BalanceDelta memory delta = IPoolManager(poolManager).swap(
            context.key,
            context.params,
            context.hookData
        );

        _handleSettlement(context, delta);

        return abi.encode(delta);
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
            bool ok = IERC20(currency).transferFrom(payer, poolManager, amount);
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
