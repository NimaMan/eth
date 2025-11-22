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

    // Command constants
    uint256 private constant CMD_V4_SWAP = 0x01;
    uint256 private constant CMD_V2_SWAP = 0x02;

    function execute(bytes calldata commands, bytes[] calldata inputs) external payable {
        if (_entered) revert RouterReentrant();
        _entered = true;
        uint256 previousNative = _nativeBuffer;
        _nativeBuffer = msg.value;

        for (uint256 i = 0; i < commands.length; i++) {
            uint256 command = uint8(commands[i]);
            bytes calldata input = inputs[i];
            _dispatch(command, input);
        }

        if (_nativeBuffer != 0) revert NativeNotFullyConsumed();
        _nativeBuffer = previousNative;
        _entered = false;
    }

    function _dispatch(uint256 command, bytes calldata input) internal {
        if (command == CMD_V4_SWAP) {
            _v4Swap(input);
        } else if (command == CMD_V2_SWAP) {
            _v2Swap(input);
        } else {
            revert("Invalid command");
        }
    }

    function _v4Swap(bytes calldata input) internal {
        // Existing V4 logic logic moved here, decoding input to determine single or multi-hop
        // For now, we assume the input encodes the operation type (single/multi) and the params.
        // This requires a slight adjustment to how we encode the V4 payload.
        // Let's reuse the existing encoding structure: (uint8 op, bytes memory payload)
        
        // However, since we are inside the router, we need to call `unlock` on the PoolManager.
        // The `unlockCallback` will be called back.
        
        // To keep state context, we can pass the input directly to the callback via the data.
        // Or we can optimize. For V4, we need the callback.
        
        bytes memory response = IPoolManager(poolManager).unlock(input);
        // We can decode response if needed, but typically we check slippage inside the callback or after.
        // The existing logic returned delta. Here we might consume it or settle it.
    }

    function _v2Swap(bytes calldata input) internal {
        (
            uint256 amountIn,
            uint256 amountOutMin,
            address[] memory path,
            address recipient
        ) = abi.decode(input, (uint256, uint256, address[], address));

        // Transfer tokens from user to router
        IERC20(path[0]).transferFrom(msg.sender, address(this), amountIn);

        // Approve V2 Router
        // Note: In production, use a constant or look up via mapping
        address v2Router = 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D; 
        IERC20(path[0]).approve(v2Router, amountIn);

        // Execute Swap
        // Using low-level call to avoid interface dependency for now
        (bool success, bytes memory returndata) = v2Router.call(
            abi.encodeWithSignature(
                "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
                amountIn,
                amountOutMin,
                path,
                recipient,
                block.timestamp
            )
        );
        if (!success) {
            if (returndata.length > 0) {
                assembly {
                    let returndata_size := mload(returndata)
                    revert(add(32, returndata), returndata_size)
                }
            } else {
                revert("V2 Swap Failed");
            }
        }
    }

    // Keep existing functions for backward compatibility during refactor if needed,
    // but preferably we switch to execute.
    // For now, I will comment out the old external functions or repurpose them.

    function unlockCallback(bytes calldata data) external override returns (bytes memory) {
        if (msg.sender != poolManager) revert UnauthorizedPoolManager();

        // Decode the data passed from _v4Swap -> unlock
        // The data should contain the swap details.
        
        (uint8 op, bytes memory payload) = abi.decode(data, (uint8, bytes));
        
        if (op == 0) {
             (SwapExactInputSingleParams memory request, address singleInitiator) = abi.decode(
                payload,
                (SwapExactInputSingleParams, address)
            );
            BalanceDelta memory delta = _executeSingle(request, singleInitiator);
            return abi.encode(delta);
        }
        
        // ... (MultiHop logic) ...
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
        try IPoolManager(poolManager).swap(context.key, context.params, context.hookData) returns (
            BalanceDelta memory swapDelta
        ) {
            delta = swapDelta;
        } catch (bytes memory reason) {
            if (reason.length == 0) revert();
            assembly {
                revert(add(reason, 0x20), mload(reason))
            }
        }
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
            BalanceDelta memory delta;
            try IPoolManager(poolManager).swap(context.key, context.params, context.hookData) returns (
                BalanceDelta memory hopDelta
            ) {
                delta = hopDelta;
            } catch (bytes memory reason) {
                if (reason.length == 0) revert();
                assembly {
                    revert(add(reason, 0x20), mload(reason))
                }
            }
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
        if (delta.amount0 > 0) {
            _settleCurrency(context.sender, context.key.currency0, uint256(int256(delta.amount0)));
        } else if (delta.amount0 < 0) {
            _takeCurrency(context.recipient, context.key.currency0, uint256(int256(-delta.amount0)));
        }

        if (delta.amount1 > 0) {
            _settleCurrency(context.sender, context.key.currency1, uint256(int256(delta.amount1)));
        } else if (delta.amount1 < 0) {
            _takeCurrency(context.recipient, context.key.currency1, uint256(int256(-delta.amount1)));
        }
    }

    function _settleCurrency(address payer, address currency, uint256 amount) internal {
        if (amount == 0) return;

        IPoolManager manager = IPoolManager(poolManager);

        if (currency == address(0)) {
            if (_nativeBuffer < amount) revert InsufficientNativeLiquidity();
            _nativeBuffer -= amount;
            manager.settle{value: amount}(address(0));
        } else {
            bool ok;
            if (payer == address(this)) {
                ok = IERC20(currency).transfer(poolManager, amount);
            } else {
                ok = IERC20(currency).transferFrom(payer, poolManager, amount);
            }
            if (!ok) revert ERC20TransferFailed();
            manager.settle(currency);
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
