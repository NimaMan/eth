// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IBalancerVault} from "./interfaces/IBalancerVault.sol";
import {ICurvePool} from "./interfaces/ICurvePool.sol";
import {IERC20} from "./interfaces/IERC20.sol";
import {IHookAdapter} from "./interfaces/IHookAdapter.sol";
import {ILockCallback} from "./interfaces/ILockCallback.sol";
import {IPoolManager} from "./interfaces/IPoolManager.sol";
import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
import {SafeTransferLib} from "./libraries/SafeTransferLib.sol";
import {
    AdapterConfig,
    BalanceDelta,
    CMD_BALANCER_FLASH_LOAN,
    CMD_BALANCER_SWAP,
    CMD_COINBASE_TIP,
    CMD_CURVE_SWAP,
    CMD_PERMIT2_TRANSFER_FROM,
    CMD_SUSHISWAP,
    CMD_SWEEP,
    CMD_TRANSFER_FROM,
    CMD_V2_SWAP,
    CMD_V3_SWAP,
    CMD_V4_SWAP,
    PoolKey,
    SwapParams
} from "./types/SharedTypes.sol";
import {
    AdapterMissing,
    CommandLengthMismatch,
    EmptyPath,
    InvalidCommand,
    InvalidTransferFromInput,
    MissingPoolManager,
    ReentrantCall,
    SlippageCheckFailed,
    SweepInsufficientBalance,
    UnauthorizedCallback,
    V2SwapFailed,
    V3SwapFailed,
    CurveSwapFailed,
    CoinbaseTipBlockMismatch,
    InvalidCoinbaseTipInput
} from "./types/Errors.sol";

contract BaygusRouter is ILockCallback {
    using SafeTransferLib for address;

    uint8 private constant CALLBACK_SINGLE = 0;
    uint8 private constant CALLBACK_PATH = 1;

    address public immutable poolManager;
    AdapterConfig public adapters;

    uint256 private _entered;

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

    struct SwapContext {
        address payer;
        address recipient;
        PoolKey key;
        SwapParams params;
        bytes hookData;
        address hookAdapter;
        int128 minAmount0;
        int128 minAmount1;
    }

    constructor(address poolManager_, AdapterConfig memory adapters_) {
        if (poolManager_ == address(0)) revert MissingPoolManager();
        poolManager = poolManager_;
        adapters = adapters_;
    }

    modifier nonReentrant() {
        if (_entered == 1) revert ReentrantCall();
        _entered = 1;
        _;
        _entered = 0;
    }

    function execute(bytes calldata commands, bytes[] calldata inputs)
        external
        payable
        nonReentrant
        returns (bytes[] memory results)
    {
        uint256 startingNativeBalance = address(this).balance - msg.value;
        bytes memory commandBytes = commands;
        bytes[] memory inputBytes = inputs;
        results = _executeCommands(commandBytes, inputBytes, msg.sender);
        _refundNative(startingNativeBalance, msg.sender);
    }

    function swapExactInputSingle(SwapExactInputSingleParams calldata request)
        external
        payable
        nonReentrant
        returns (BalanceDelta memory delta)
    {
        uint256 startingNativeBalance = address(this).balance - msg.value;
        delta = _unlockSingle(msg.sender, request);
        _refundNative(startingNativeBalance, msg.sender);
    }

    function swapExactInputPath(MultiHopParams calldata request)
        external
        payable
        nonReentrant
        returns (BalanceDelta memory delta)
    {
        uint256 startingNativeBalance = address(this).balance - msg.value;
        delta = _unlockPath(msg.sender, request);
        _refundNative(startingNativeBalance, msg.sender);
    }

    function unlockCallback(bytes calldata data) external override returns (bytes memory) {
        if (msg.sender != poolManager) revert UnauthorizedCallback();

        (uint8 kind, bytes memory payload) = abi.decode(data, (uint8, bytes));
        if (kind == CALLBACK_SINGLE) {
            (address payer, SwapExactInputSingleParams memory request) =
                abi.decode(payload, (address, SwapExactInputSingleParams));
            return abi.encode(_executeSingle(payer, request));
        }
        if (kind == CALLBACK_PATH) {
            (address payer, MultiHopParams memory request) = abi.decode(payload, (address, MultiHopParams));
            return abi.encode(_executePath(payer, request));
        }

        revert InvalidCommand(kind);
    }

    function receiveFlashLoan(
        address[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external {
        if (msg.sender != adapters.balancerVault) revert UnauthorizedCallback();

        (bytes memory commands, bytes[] memory inputs) = abi.decode(userData, (bytes, bytes[]));
        _executeCommands(commands, inputs, address(this));

        for (uint256 i = 0; i < tokens.length;) {
            tokens[i].safeTransfer(adapters.balancerVault, amounts[i] + feeAmounts[i]);
            unchecked {
                ++i;
            }
        }
    }

    function _executeCommands(bytes memory commands, bytes[] memory inputs, address payer)
        internal
        returns (bytes[] memory results)
    {
        if (commands.length != inputs.length) revert CommandLengthMismatch();

        results = new bytes[](commands.length);
        for (uint256 i = 0; i < commands.length;) {
            results[i] = _dispatch(uint8(commands[i]), inputs[i], payer);
            unchecked {
                ++i;
            }
        }
    }

    function _dispatch(uint8 command, bytes memory input, address payer) internal returns (bytes memory result) {
        if (command == CMD_TRANSFER_FROM) {
            _transferFrom(input, payer);
        } else if (command == CMD_V2_SWAP) {
            _v2Swap(input, adapters.uniswapV2Router, command);
        } else if (command == CMD_SUSHISWAP) {
            _v2Swap(input, adapters.sushiswapRouter, command);
        } else if (command == CMD_V3_SWAP) {
            _v3Swap(input);
        } else if (command == CMD_CURVE_SWAP) {
            _curveSwap(input);
        } else if (command == CMD_BALANCER_SWAP) {
            _balancerSwap(input);
        } else if (command == CMD_SWEEP) {
            _sweep(input);
        } else if (command == CMD_BALANCER_FLASH_LOAN) {
            _balancerFlashLoan(input);
        } else if (command == CMD_V4_SWAP) {
            result = IPoolManager(poolManager).unlock(input);
        } else if (command == CMD_PERMIT2_TRANSFER_FROM) {
            revert AdapterMissing(command);
        } else if (command == CMD_COINBASE_TIP) {
            _coinbaseTip(input);
        } else {
            revert InvalidCommand(command);
        }
    }

    function _unlockSingle(address payer, SwapExactInputSingleParams calldata request)
        internal
        returns (BalanceDelta memory delta)
    {
        bytes memory response =
            IPoolManager(poolManager).unlock(abi.encode(CALLBACK_SINGLE, abi.encode(payer, request)));
        delta = abi.decode(response, (BalanceDelta));
    }

    function _unlockPath(address payer, MultiHopParams calldata request) internal returns (BalanceDelta memory delta) {
        bytes memory response = IPoolManager(poolManager).unlock(abi.encode(CALLBACK_PATH, abi.encode(payer, request)));
        delta = abi.decode(response, (BalanceDelta));
    }

    function _executeSingle(address payer, SwapExactInputSingleParams memory request)
        internal
        returns (BalanceDelta memory delta)
    {
        SwapContext memory context = SwapContext({
            payer: payer,
            recipient: request.recipient,
            key: request.key,
            params: request.params,
            hookData: request.hookData,
            hookAdapter: request.hookAdapter,
            minAmount0: request.minAmount0,
            minAmount1: request.minAmount1
        });

        _beforeSwap(context);
        delta = _decodeBalanceDelta(IPoolManager(poolManager).swap(context.key, context.params, context.hookData));
        _validateDelta(context, delta);
        _settleDelta(context.payer, context.recipient, context.key, delta);
        _afterSwap(context, delta);
    }

    function _executePath(address payer, MultiHopParams memory request)
        internal
        returns (BalanceDelta memory finalDelta)
    {
        if (request.hops.length == 0) revert EmptyPath();

        address[] memory currencies = new address[](request.hops.length * 2);
        int256[] memory netAmounts = new int256[](request.hops.length * 2);
        uint256 netCount;
        address currentPayer = payer;

        for (uint256 i = 0; i < request.hops.length;) {
            Hop memory hop = request.hops[i];
            address recipient = i + 1 == request.hops.length ? request.recipient : address(this);
            SwapContext memory context = SwapContext({
                payer: currentPayer,
                recipient: recipient,
                key: hop.key,
                params: hop.params,
                hookData: hop.hookData,
                hookAdapter: hop.hookAdapter,
                minAmount0: hop.minAmount0,
                minAmount1: hop.minAmount1
            });

            _beforeSwap(context);
            BalanceDelta memory delta =
                _decodeBalanceDelta(IPoolManager(poolManager).swap(context.key, context.params, context.hookData));
            _validateDelta(context, delta);
            _afterSwap(context, delta);

            netCount = _accumulate(currencies, netAmounts, netCount, context.key.currency0, delta.amount0);
            netCount = _accumulate(currencies, netAmounts, netCount, context.key.currency1, delta.amount1);
            finalDelta = delta;
            currentPayer = address(this);

            unchecked {
                ++i;
            }
        }

        if (request.finalMinAmount0 != 0 && finalDelta.amount0 < request.finalMinAmount0) {
            revert SlippageCheckFailed(2, finalDelta.amount0, request.finalMinAmount0);
        }
        if (request.finalMinAmount1 != 0 && finalDelta.amount1 < request.finalMinAmount1) {
            revert SlippageCheckFailed(3, finalDelta.amount1, request.finalMinAmount1);
        }

        for (uint256 i = 0; i < netCount;) {
            int256 amount = netAmounts[i];
            if (amount < 0) {
                _settleCurrency(payer, currencies[i], _absNet(amount));
            } else if (amount > 0) {
                _takeCurrency(request.recipient, currencies[i], _absNet(amount));
            }
            unchecked {
                ++i;
            }
        }
    }

    function _transferFrom(bytes memory input, address defaultFrom) internal {
        address token;
        address from;
        uint256 amount;

        if (input.length == 64) {
            (token, amount) = abi.decode(input, (address, uint256));
            from = defaultFrom;
        } else if (input.length == 96) {
            (token, from, amount) = abi.decode(input, (address, address, uint256));
        } else {
            revert InvalidTransferFromInput();
        }

        token.safeTransferFrom(from, address(this), amount);
    }

    function _v2Swap(bytes memory input, address router, uint8 command) internal {
        if (router == address(0)) revert AdapterMissing(command);

        (uint256 amountIn, uint256 amountOutMin, address[] memory path, address recipient) =
            abi.decode(input, (uint256, uint256, address[], address));
        if (path.length < 2) revert EmptyPath();
        if (amountIn == 0) {
            amountIn = IERC20(path[0]).balanceOf(address(this));
        }

        _approveIfNeeded(path[0], router, amountIn);
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
            assembly {
                revert(add(data, 0x20), mload(data))
            }
        }
    }

    function _v3Swap(bytes memory input) internal {
        if (adapters.uniswapV3Router == address(0)) revert AdapterMissing(CMD_V3_SWAP);

        ISwapRouter.ExactInputSingleParams memory params = abi.decode(input, (ISwapRouter.ExactInputSingleParams));
        if (params.amountIn == 0) {
            params.amountIn = IERC20(params.tokenIn).balanceOf(address(this));
        }

        _approveIfNeeded(params.tokenIn, adapters.uniswapV3Router, params.amountIn);
        try ISwapRouter(adapters.uniswapV3Router).exactInputSingle(params) returns (uint256) {
            return;
        } catch (bytes memory reason) {
            if (reason.length == 0) revert V3SwapFailed();
            assembly {
                revert(add(reason, 0x20), mload(reason))
            }
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
            uint256 minDy,
            bool useUnderlying
        ) = abi.decode(input, (address, address, address, address, int128, int128, uint256, uint256, bool));
        if (dx == 0) {
            dx = IERC20(tokenIn).balanceOf(address(this));
        }

        _approveIfNeeded(tokenIn, pool, dx);
        uint256 beforeBalance = IERC20(tokenOut).balanceOf(address(this));
        if (useUnderlying) {
            ICurvePool(pool).exchange_underlying(i, j, dx, minDy);
        } else {
            ICurvePool(pool).exchange(i, j, dx, minDy);
        }

        uint256 output = IERC20(tokenOut).balanceOf(address(this)) - beforeBalance;
        if (output < minDy) revert CurveSwapFailed();
        if (output != 0) tokenOut.safeTransfer(recipient, output);
    }

    function _balancerSwap(bytes memory input) internal {
        if (adapters.balancerVault == address(0)) revert AdapterMissing(CMD_BALANCER_SWAP);

        (bytes32 poolId, address assetIn, address assetOut, address recipient, uint256 amount, uint256 limit) =
            abi.decode(input, (bytes32, address, address, address, uint256, uint256));
        if (amount == 0) {
            amount = IERC20(assetIn).balanceOf(address(this));
        }

        _approveIfNeeded(assetIn, adapters.balancerVault, amount);
        IBalancerVault.SingleSwap memory singleSwap = IBalancerVault.SingleSwap({
            poolId: poolId,
            kind: IBalancerVault.SwapKind.GIVEN_IN,
            assetIn: assetIn,
            assetOut: assetOut,
            amount: amount,
            userData: ""
        });
        IBalancerVault.FundManagement memory funds = IBalancerVault.FundManagement({
            sender: address(this), fromInternalBalance: false, recipient: payable(recipient), toInternalBalance: false
        });

        IBalancerVault(adapters.balancerVault).swap(singleSwap, funds, limit, block.timestamp);
    }

    function _balancerFlashLoan(bytes memory input) internal {
        if (adapters.balancerVault == address(0)) revert AdapterMissing(CMD_BALANCER_FLASH_LOAN);

        (address[] memory tokens, uint256[] memory amounts, bytes memory userData) =
            abi.decode(input, (address[], uint256[], bytes));
        IBalancerVault(adapters.balancerVault).flashLoan(address(this), tokens, amounts, userData);
    }

    function _sweep(bytes memory input) internal {
        (address token, address recipient, uint256 minimumAmount) = abi.decode(input, (address, address, uint256));
        uint256 balance;
        if (token == address(0)) {
            balance = address(this).balance;
            if (balance < minimumAmount) revert SweepInsufficientBalance(token, balance, minimumAmount);
            if (balance != 0) recipient.safeTransferEth(balance);
        } else {
            balance = IERC20(token).balanceOf(address(this));
            if (balance < minimumAmount) revert SweepInsufficientBalance(token, balance, minimumAmount);
            if (balance != 0) token.safeTransfer(recipient, balance);
        }
    }

    function _coinbaseTip(bytes memory input) internal {
        uint256 amount;
        uint256 minBlock;
        uint256 maxBlock;

        if (input.length == 32) {
            amount = abi.decode(input, (uint256));
        } else if (input.length == 96) {
            (amount, minBlock, maxBlock) = abi.decode(input, (uint256, uint256, uint256));
        } else {
            revert InvalidCoinbaseTipInput();
        }

        if (block.number < minBlock || (maxBlock != 0 && block.number > maxBlock)) {
            revert CoinbaseTipBlockMismatch(block.number, minBlock, maxBlock);
        }
        if (amount != 0) {
            address(block.coinbase).safeTransferEth(amount);
        }
    }

    function _settleDelta(address payer, address recipient, PoolKey memory key, BalanceDelta memory delta) internal {
        if (delta.amount0 < 0) {
            _settleCurrency(payer, key.currency0, _absDelta(delta.amount0));
        } else if (delta.amount0 > 0) {
            _takeCurrency(recipient, key.currency0, _absDelta(delta.amount0));
        }

        if (delta.amount1 < 0) {
            _settleCurrency(payer, key.currency1, _absDelta(delta.amount1));
        } else if (delta.amount1 > 0) {
            _takeCurrency(recipient, key.currency1, _absDelta(delta.amount1));
        }
    }

    function _settleCurrency(address payer, address currency, uint256 amount) internal {
        if (amount == 0) return;
        if (currency == address(0)) {
            IPoolManager(poolManager).settle{value: amount}();
        } else {
            IPoolManager(poolManager).sync(currency);
            if (payer == address(this)) {
                currency.safeTransfer(poolManager, amount);
            } else {
                currency.safeTransferFrom(payer, poolManager, amount);
            }
            IPoolManager(poolManager).settle();
        }
    }

    function _takeCurrency(address recipient, address currency, uint256 amount) internal {
        if (amount == 0) return;
        IPoolManager(poolManager).take(currency, recipient, amount);
    }

    function _beforeSwap(SwapContext memory context) internal {
        if (context.hookAdapter == address(0)) return;
        IHookAdapter(context.hookAdapter)
            .beforeSwap(context.payer, context.recipient, context.key, context.params, context.hookData);
    }

    function _afterSwap(SwapContext memory context, BalanceDelta memory delta) internal {
        if (context.hookAdapter == address(0)) return;
        IHookAdapter(context.hookAdapter)
            .afterSwap(
                context.payer,
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

    function _accumulate(
        address[] memory currencies,
        int256[] memory netAmounts,
        uint256 count,
        address currency,
        int128 delta
    ) internal pure returns (uint256) {
        if (delta == 0) return count;
        for (uint256 i = 0; i < count;) {
            if (currencies[i] == currency) {
                netAmounts[i] += int256(delta);
                return count;
            }
            unchecked {
                ++i;
            }
        }
        currencies[count] = currency;
        netAmounts[count] = int256(delta);
        return count + 1;
    }

    function _decodeBalanceDelta(int256 packed) internal pure returns (BalanceDelta memory delta) {
        // PoolManager packs each signed amount into exactly one int128 lane.
        // forge-lint: disable-next-line(unsafe-typecast)
        delta.amount0 = int128(packed >> 128);
        // PoolManager packs each signed amount into exactly one int128 lane.
        // forge-lint: disable-next-line(unsafe-typecast)
        delta.amount1 = int128(packed);
    }

    function _absDelta(int128 value) internal pure returns (uint256) {
        if (value < 0) {
            // A BalanceDelta lane is int128, so its absolute value fits uint128.
            // forge-lint: disable-next-line(unsafe-typecast)
            return uint256(uint128(-value));
        }
        // A non-negative int128 always fits uint128.
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint256(uint128(value));
    }

    function _absNet(int256 value) internal pure returns (uint256) {
        return uint256(value < 0 ? -value : value);
    }

    function _approveIfNeeded(address token, address spender, uint256 amount) internal {
        if (amount == 0) return;
        if (IERC20(token).allowance(address(this), spender) < amount) {
            token.safeApprove(spender, type(uint256).max);
        }
    }

    function _refundNative(uint256 startingNativeBalance, address recipient) internal {
        uint256 currentBalance = address(this).balance;
        if (currentBalance > startingNativeBalance) {
            recipient.safeTransferEth(currentBalance - startingNativeBalance);
        }
    }

    receive() external payable {}
}
