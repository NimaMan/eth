// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ILockCallback} from "./interfaces/ILockCallback.sol";
import {IPoolManager} from "./interfaces/IPoolManager.sol";
import {IERC20} from "./interfaces/IERC20.sol";
import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
import {IHookAdapter} from "./interfaces/IHookAdapter.sol";
import {PoolKey, SwapParams, BalanceDelta, CMD_V4_SWAP, CMD_V2_SWAP, CMD_V3_SWAP, CMD_SUSHISWAP, CMD_CURVE_SWAP, CMD_BALANCER_SWAP, CMD_SWEEP, CMD_BALANCER_FLASH_LOAN} from "./types/SharedTypes.sol";
import {SafeTransferLib} from "./libraries/SafeTransferLib.sol";
import {InvalidCommand, V2SwapFailed, V3SwapFailed, CurveSwapFailed, CurveApproveFailed, CurveTransferFromFailed, SweepInsufficientBalance, ETHTransferFailed} from "./types/Errors.sol";
import {ICurvePool} from "./interfaces/ICurvePool.sol";
import {IBalancerVault} from "./interfaces/IBalancerVault.sol";

/// @title BaygusRouter
/// @notice Router used by the Baygus execution agent: performs Uniswap v4 PoolManager lock →
///         swap → settle flows with optional hook adapters and per-leg slippage checks.
/// @dev Designed for simulation and staged deployment; future versions will layer in multi-hop
///      support and additional venue adapters.
contract BaygusRouter is ILockCallback {
    using SafeTransferLib for address;

    address public immutable poolManager;
    address private constant UNISWAP_V2_ROUTER = 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D;
    address private constant SUSHISWAP_ROUTER = 0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F;
    address private constant BALANCER_VAULT = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;

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

    function _dispatch(uint256 command, bytes memory input) internal {
        if (command == CMD_V4_SWAP) {
            _v4Swap(input);
        } else if (command == CMD_V2_SWAP) {
            _v2Swap(input, UNISWAP_V2_ROUTER);
        } else if (command == CMD_V3_SWAP) {
            _v3Swap(input);
        } else if (command == CMD_SUSHISWAP) {
            _v2Swap(input, SUSHISWAP_ROUTER);
        } else if (command == CMD_CURVE_SWAP) {
            _curveSwap(input);
        } else if (command == CMD_BALANCER_SWAP) {
            _balancerSwap(input);
        } else if (command == CMD_SWEEP) {
            _sweep(input);
        } else if (command == CMD_BALANCER_FLASH_LOAN) {
            _balancerFlashLoan(input);
        } else {
            revert InvalidCommand();
        }
    }

    function _v4Swap(bytes memory input) internal {
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

    function _v2Swap(bytes memory input, address router) internal {
        (
            uint256 amountIn,
            uint256 amountOutMin,
            address[] memory path,
            address recipient,
            bool payerIsUser
        ) = abi.decode(input, (uint256, uint256, address[], address, bool));

        // Transfer tokens from user to router if payer is user
        if (payerIsUser) {
            path[0].safeTransferFrom(msg.sender, address(this), amountIn);
        } else if (amountIn == 0) {
            amountIn = IERC20(path[0]).balanceOf(address(this));
        }

        // Approve V2/Sushi Router
        _approveIfNecessary(path[0], router, amountIn);

        // Execute Swap
        // Using low-level call to avoid interface dependency for now
        (bool success, ) = router.call(
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
            revert V2SwapFailed();
        }
    }

    function _v3Swap(bytes memory input) internal {
        (ISwapRouter.ExactInputSingleParams memory params, bool payerIsUser) = abi.decode(input, (ISwapRouter.ExactInputSingleParams, bool));
        
        // Transfer tokenIn from user to router
        if (payerIsUser) {
            params.tokenIn.safeTransferFrom(msg.sender, address(this), params.amountIn);
        } else if (params.amountIn == 0) {
            params.amountIn = IERC20(params.tokenIn).balanceOf(address(this));
        }
        
        // Approve V3 Router
        address v3Router = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
        _approveIfNecessary(params.tokenIn, v3Router, params.amountIn);
        
        // Execute Swap
        try ISwapRouter(v3Router).exactInputSingle(params) returns (uint256 amountOut) {
            // Success
            amountOut;
        } catch {
             revert V3SwapFailed();
        }
    }

    function _curveSwap(bytes memory input) internal {
        (
            address pool,
            address tokenIn,
            address tokenOut,
            address recipient,
            int128 i,
            int128 j,
            uint256 dx,
            uint256 min_dy,
            bool useUnderlying,
            bool payerIsUser
        ) = abi.decode(input, (address, address, address, address, int128, int128, uint256, uint256, bool, bool));

        // Transfer From
        if (payerIsUser) {
            tokenIn.safeTransferFrom(msg.sender, address(this), dx);
        } else if (dx == 0) {
            dx = IERC20(tokenIn).balanceOf(address(this));
        }

        // Approve
        _approveIfNecessary(tokenIn, pool, dx);

        uint256 balanceBefore = IERC20(tokenOut).balanceOf(address(this));

        // Exchange
        bool success;
        if (useUnderlying) {
             (success, ) = pool.call(
                abi.encodeWithSignature("exchange_underlying(int128,int128,uint256,uint256)", i, j, dx, min_dy)
            );
        } else {
             (success, ) = pool.call(
                abi.encodeWithSignature("exchange(int128,int128,uint256,uint256)", i, j, dx, min_dy)
            );
        }
        
        if (!success) {
             revert CurveSwapFailed();
        }

        uint256 balanceAfter = IERC20(tokenOut).balanceOf(address(this));
        uint256 amountOut = balanceAfter - balanceBefore;

        if (amountOut > 0) {
            tokenOut.safeTransfer(recipient, amountOut);
        }
    }

    function _balancerSwap(bytes memory input) internal {
        (
            bytes32 poolId,
            address assetIn,
            address assetOut,
            address recipient,
            uint256 amount,
            uint256 limit,
            bool payerIsUser
        ) = abi.decode(input, (bytes32, address, address, address, uint256, uint256, bool));

        // Transfer From
        if (payerIsUser) {
            assetIn.safeTransferFrom(msg.sender, address(this), amount);
        } else if (amount == 0) {
            amount = IERC20(assetIn).balanceOf(address(this));
        }

        // Approve Vault
        _approveIfNecessary(assetIn, BALANCER_VAULT, amount);

        IBalancerVault.SingleSwap memory singleSwap = IBalancerVault.SingleSwap({
            poolId: poolId,
            kind: IBalancerVault.SwapKind.GIVEN_IN,
            assetIn: assetIn,
            assetOut: assetOut,
            amount: amount,
            userData: ""
        });

        IBalancerVault.FundManagement memory funds = IBalancerVault.FundManagement({
            sender: address(this),
            fromInternalBalance: false,
            recipient: payable(recipient),
            toInternalBalance: false
        });

        IBalancerVault(BALANCER_VAULT).swap(
            singleSwap,
            funds,
            limit,
            block.timestamp
        );
    }

    function _balancerFlashLoan(bytes memory input) internal {
        (address[] memory tokens, uint256[] memory amounts, bytes memory userData) = abi.decode(input, (address[], uint256[], bytes));
        IBalancerVault(BALANCER_VAULT).flashLoan(address(this), tokens, amounts, userData);
    }

    function receiveFlashLoan(
        address[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external {
        if (msg.sender != BALANCER_VAULT) revert UnauthorizedPoolManager();

        (bytes memory commands, bytes[] memory inputs) = abi.decode(userData, (bytes, bytes[]));
        
        for (uint256 i = 0; i < commands.length; i++) {
            _dispatch(uint8(commands[i]), inputs[i]);
        }

        // Repay
        for (uint256 i = 0; i < tokens.length; ++i) {
            uint256 amountToRepay = amounts[i] + feeAmounts[i];
            tokens[i].safeTransfer(BALANCER_VAULT, amountToRepay);
        }
    }

    function _sweep(bytes memory input) internal {
        (address token, address recipient, uint256 amountMinimum) = abi.decode(input, (address, address, uint256));
        
        uint256 balance;
        if (token == address(0)) {
            balance = address(this).balance;
            if (balance > 0) {
                SafeTransferLib.safeTransferETH(recipient, balance);
            }
        } else {
            balance = IERC20(token).balanceOf(address(this));
            if (balance > 0) {
                token.safeTransfer(recipient, balance);
            }
        }
        
        if (balance < amountMinimum) {
            revert SweepInsufficientBalance();
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
            }
            else {
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

    function _approveIfNecessary(address token, address spender, uint256 amount) internal {
        if (IERC20(token).allowance(address(this), spender) < amount) {
            token.safeApprove(spender, type(uint256).max);
        }
    }

    receive() external payable {}
}
