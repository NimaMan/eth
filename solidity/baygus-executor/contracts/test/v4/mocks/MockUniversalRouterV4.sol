// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../../src/interfaces/IERC20.sol";
import {SafeTransferLib} from "../../../src/libraries/SafeTransferLib.sol";

interface IMockPermit2Transfer {
    function transferFrom(address from, address to, uint160 amount, address token) external;
}

contract MockUniversalRouterV4 {
    using SafeTransferLib for address;

    struct PoolKey {
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }

    struct ExactInputSingleParams {
        PoolKey poolKey;
        bool zeroForOne;
        uint128 amountIn;
        uint128 amountOutMinimum;
        bytes hookData;
        uint256 minHopPriceX36;
    }

    uint8 private constant COMMAND_V4_SWAP = 0x10;
    uint8 private constant ACTION_SWAP_EXACT_IN_SINGLE = 0x06;
    uint8 private constant ACTION_SETTLE = 0x0b;
    uint8 private constant ACTION_TAKE = 0x0e;

    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable permit2;

    uint256 public nextTokenOut;
    uint256 public nextEthOut;

    address public lastCaller;
    address public lastInputCurrency;
    address public lastOutputCurrency;
    address public lastRecipient;
    address public lastHooks;
    uint24 public lastFee;
    int24 public lastTickSpacing;
    bool public lastZeroForOne;
    uint256 public lastAmountIn;
    uint256 public lastMinAmountOut;
    uint256 public lastValue;
    uint256 public lastDeadline;
    bytes public lastHookData;

    constructor(address permit2_) {
        permit2 = permit2_;
    }

    receive() external payable {}

    function setNextTokenOut(uint256 amount) external {
        nextTokenOut = amount;
    }

    function setNextEthOut(uint256 amount) external {
        nextEthOut = amount;
    }

    function execute(bytes calldata commands, bytes[] calldata inputs, uint256 deadline) external payable {
        require(commands.length == 1 && uint8(commands[0]) == COMMAND_V4_SWAP, "MockV4Router: command");
        require(inputs.length == 1, "MockV4Router: inputs");
        (bytes memory actions, bytes[] memory params) = abi.decode(inputs[0], (bytes, bytes[]));
        require(actions.length == 3, "MockV4Router: actions length");
        require(uint8(actions[0]) == ACTION_SWAP_EXACT_IN_SINGLE, "MockV4Router: swap action");
        require(uint8(actions[1]) == ACTION_SETTLE, "MockV4Router: settle action");
        require(uint8(actions[2]) == ACTION_TAKE, "MockV4Router: take action");
        require(params.length == 3, "MockV4Router: params length");

        ExactInputSingleParams memory swap = abi.decode(params[0], (ExactInputSingleParams));
        (address inputCurrency, uint256 settleAmount, bool payerIsUser) =
            abi.decode(params[1], (address, uint256, bool));
        (address outputCurrency, address recipient,) = abi.decode(params[2], (address, address, uint256));

        require(payerIsUser, "MockV4Router: payer");
        require(settleAmount == swap.amountIn, "MockV4Router: settle amount");

        lastCaller = msg.sender;
        lastInputCurrency = inputCurrency;
        lastOutputCurrency = outputCurrency;
        lastRecipient = recipient;
        lastHooks = swap.poolKey.hooks;
        lastFee = swap.poolKey.fee;
        lastTickSpacing = swap.poolKey.tickSpacing;
        lastZeroForOne = swap.zeroForOne;
        lastAmountIn = settleAmount;
        lastMinAmountOut = swap.amountOutMinimum;
        lastValue = msg.value;
        lastDeadline = deadline;
        lastHookData = swap.hookData;

        if (inputCurrency == address(0)) {
            _executeEthForToken(outputCurrency, recipient, settleAmount, swap.amountOutMinimum);
        } else if (outputCurrency == address(0)) {
            _executeTokenForEth(inputCurrency, recipient, settleAmount, swap.amountOutMinimum);
        } else {
            revert("MockV4Router: unsupported route");
        }
    }

    function _executeEthForToken(address outputCurrency, address recipient, uint256 amountIn, uint256 minAmountOut)
        private
    {
        require(msg.value == amountIn, "MockV4Router: value");
        require(nextTokenOut >= minAmountOut, "MockV4Router: min token out");
        outputCurrency.safeTransfer(recipient, nextTokenOut);
    }

    function _executeTokenForEth(address inputCurrency, address recipient, uint256 amountIn, uint256 minAmountOut)
        private
    {
        require(msg.value == 0, "MockV4Router: unexpected value");
        require(nextEthOut >= minAmountOut, "MockV4Router: min eth out");
        IMockPermit2Transfer(permit2)
            .transferFrom(msg.sender, address(this), uint160(swapAmountIn(amountIn)), inputCurrency);
        recipient.safeTransferEth(nextEthOut);
    }

    function swapAmountIn(uint256 amountIn) private pure returns (uint128) {
        require(amountIn <= type(uint128).max, "MockV4Router: amount");
        // Casting is safe because the range check above enforces uint128 max.
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint128(amountIn);
    }

    function tokenBalance(address token) external view returns (uint256) {
        return IERC20(token).balanceOf(address(this));
    }
}
