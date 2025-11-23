// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ILockCallback} from "./interfaces/ILockCallback.sol";
import {IPoolManager} from "./interfaces/IPoolManager.sol";
import {IERC20} from "./interfaces/IERC20.sol";
import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
import {IHookAdapter} from "./interfaces/IHookAdapter.sol";
import {PoolKey, SwapParams, BalanceDelta, CMD_V4_SWAP, CMD_V2_SWAP, CMD_V3_SWAP, CMD_SUSHISWAP, CMD_CURVE_SWAP, CMD_BALANCER_SWAP, CMD_SWEEP, CMD_BALANCER_FLASH_LOAN, CMD_PERMIT2_TRANSFER_FROM, CMD_TRANSFER_FROM} from "./types/SharedTypes.sol";
import {SafeTransferLib} from "./libraries/SafeTransferLib.sol";
import {InvalidCommand, V2SwapFailed, V3SwapFailed, CurveSwapFailed, CurveApproveFailed, CurveTransferFromFailed, SweepInsufficientBalance, ETHTransferFailed} from "./types/Errors.sol";
import {ICurvePool} from "./interfaces/ICurvePool.sol";
import {IBalancerVault} from "./interfaces/IBalancerVault.sol";
import {IPermit2} from "./interfaces/IPermit2.sol";

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
    address private constant PERMIT2 = 0x000000000022D473030F116dDEE9F6B43aC78BA3;
    address private constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564; 

    // Transient storage slots
    uint256 private constant TSLOT_ENTERED = 0;
    uint256 private constant TSLOT_NATIVE_BUFFER = 1;

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
        uint256 entered;
        assembly { entered := tload(TSLOT_ENTERED) }
        if (entered == 1) revert RouterReentrant();
        
        assembly { tstore(TSLOT_ENTERED, 1) }
        
        uint256 previousNative;
        assembly { previousNative := tload(TSLOT_NATIVE_BUFFER) }
        
        assembly { tstore(TSLOT_NATIVE_BUFFER, callvalue()) }

        for (uint256 i = 0; i < commands.length; i++) {
            uint256 command = uint8(commands[i]);
            bytes calldata input = inputs[i];
            _dispatch(command, input);
        }

        uint256 remainingNative;
        assembly { remainingNative := tload(TSLOT_NATIVE_BUFFER) }
        if (remainingNative != 0) revert NativeNotFullyConsumed();
        
        assembly {
            tstore(TSLOT_NATIVE_BUFFER, previousNative)
            tstore(TSLOT_ENTERED, 0)
        }
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
        } else if (command == CMD_PERMIT2_TRANSFER_FROM) {
            _permit2TransferFrom(input);
        } else if (command == CMD_TRANSFER_FROM) {
            _transferFrom(input);
        } else {
            revert InvalidCommand();
        }
    }

    function _transferFrom(bytes memory input) internal {
        (address token, uint256 amount) = abi.decode(input, (address, uint256));
        token.safeTransferFrom(msg.sender, address(this), amount);
    }

    function _v4Swap(bytes memory input) internal {
        IPoolManager(poolManager).unlock(input);
    }

    function _v2Swap(bytes memory input, address router) internal {
        (
            uint256 amountIn,
            uint256 amountOutMin,
            address[] memory path,
            address recipient
        ) = abi.decode(input, (uint256, uint256, address[], address));

        if (amountIn == 0) {
            amountIn = IERC20(path[0]).balanceOf(address(this));
        }

        _approveIfNecessary(path[0], router, amountIn);

        (bool success, bytes memory data) = router.call(
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
            if (data.length == 0) revert V2SwapFailed();
            assembly { revert(add(data, 0x20), mload(data)) }
        }
    }

    function _v3Swap(bytes memory input) internal {
        ISwapRouter.ExactInputSingleParams memory params = abi.decode(input, (ISwapRouter.ExactInputSingleParams));
        
        if (params.amountIn == 0) {
            params.amountIn = IERC20(params.tokenIn).balanceOf(address(this));
        }
        
        _approveIfNecessary(params.tokenIn, UNISWAP_V3_ROUTER, params.amountIn);
        
        try ISwapRouter(UNISWAP_V3_ROUTER).exactInputSingle(params) returns (uint256 amountOut) {
            amountOut;
        } catch (bytes memory reason) {
             if (reason.length == 0) revert V3SwapFailed();
             assembly { revert(add(reason, 0x20), mload(reason)) }
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
            bool useUnderlying
        ) = abi.decode(input, (address, address, address, address, int128, int128, uint256, uint256, bool));

        if (dx == 0) {
            dx = IERC20(tokenIn).balanceOf(address(this));
        }

        _approveIfNecessary(tokenIn, pool, dx);

        uint256 balanceBefore = IERC20(tokenOut).balanceOf(address(this));

        bool success;
        bytes memory data;
        if (useUnderlying) {
             (success, data) = pool.call(
                abi.encodeWithSignature("exchange_underlying(int128,int128,uint256,uint256)", i, j, dx, min_dy)
            );
        } else {
             (success, data) = pool.call(
                abi.encodeWithSignature("exchange(int128,int128,uint256,uint256)", i, j, dx, min_dy)
            );
        }
        
        if (!success) {
             if (data.length == 0) revert CurveSwapFailed();
             assembly { revert(add(data, 0x20), mload(data)) }
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
            uint256 limit
        ) = abi.decode(input, (bytes32, address, address, address, uint256, uint256));

        if (amount == 0) {
            amount = IERC20(assetIn).balanceOf(address(this));
        }

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

    function _permit2TransferFrom(bytes memory input) internal {
        (
            IPermit2.SignatureTransferDetails memory transferDetails,
            address owner,
            bytes memory signature
        ) = abi.decode(input, (IPermit2.SignatureTransferDetails, address, bytes));
        
        IPermit2(PERMIT2).permitTransferFrom(
            transferDetails,
            owner,
            signature,
            transferDetails.amount // The amount to transfer, used as 'value' param
        );
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

        if (finalMinAmount0 != 0 && finalDelta.amount0 > finalMinAmount0) {
            revert SlippageCheckFailed(2, finalDelta.amount0, finalMinAmount0);
        }
        if (finalMinAmount1 != 0 && finalDelta.amount1 > finalMinAmount1) {
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
            uint256 currentBuffer;
            assembly { currentBuffer := tload(TSLOT_NATIVE_BUFFER) }
            
            if (currentBuffer < amount) revert InsufficientNativeLiquidity();
            
            assembly { tstore(TSLOT_NATIVE_BUFFER, sub(currentBuffer, amount)) }
            
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
        // In V4, user gain is negative, user pay is positive.
        // If minAmount is negative (minimum output), we want delta <= minAmount (e.g. -150 <= -100).
        // If delta > minAmount (e.g. -90 > -100), it means we received less (absolute), so revert.
        // If minAmount is positive (max input), we want delta <= minAmount (e.g. 90 <= 100).
        // If delta > minAmount (e.g. 110 > 100), we paid too much, so revert.
        if (context.minAmount0 != 0 && delta.amount0 > context.minAmount0) {
            revert SlippageCheckFailed(0, delta.amount0, context.minAmount0);
        }
        if (context.minAmount1 != 0 && delta.amount1 > context.minAmount1) {
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