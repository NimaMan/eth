// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../interfaces/IERC20.sol";
import {SafeTransferLib} from "../libraries/SafeTransferLib.sol";

interface IUniversalRouter {
    function execute(bytes calldata commands, bytes[] calldata inputs, uint256 deadline) external payable;
}

interface IPermit2 {
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

contract UniswapV4TradingVault {
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

    error AmountTooLarge(uint256 amount);
    error DeadlineExpired(uint256 deadline);
    error HookDataWithoutHook();
    error HookNotAllowed(address hook);
    error InsufficientOutput(uint256 actual, uint256 minimum);
    error InvalidPoolKey();
    error ReentrantCall();
    error Unauthorized();
    error ZeroAddress();
    error ZeroAmount();

    // forge-lint: disable-next-line(screaming-snake-case-const)
    uint8 private constant COMMAND_V4_SWAP = 0x10;
    // forge-lint: disable-next-line(screaming-snake-case-const)
    uint8 private constant ACTION_SWAP_EXACT_IN_SINGLE = 0x06;
    // forge-lint: disable-next-line(screaming-snake-case-const)
    uint8 private constant ACTION_SETTLE = 0x0b;
    // forge-lint: disable-next-line(screaming-snake-case-const)
    uint8 private constant ACTION_TAKE = 0x0e;
    uint48 private constant PERMIT2_EXPIRATION = type(uint48).max;

    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable owner;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable treasury;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable universalRouter;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable permit2;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    bool public immutable hooksAllowed;

    uint256 private _entered;

    event BoughtV4(
        address indexed token, uint256 ethIn, uint256 tokensReceived, uint256 minTokensOut, address indexed hooks
    );
    event EmergencySoldV4(
        address indexed token, uint256 tokenIn, uint256 ethReceived, uint256 minEthOut, address indexed hooks
    );
    event RescueEth(address indexed to, uint256 amount);
    event RescueToken(address indexed token, address indexed to, uint256 amount);

    constructor(address owner_, address treasury_, address universalRouter_, address permit2_, bool hooksAllowed_) {
        if (owner_ == address(0) || treasury_ == address(0) || universalRouter_ == address(0) || permit2_ == address(0))
        {
            revert ZeroAddress();
        }
        owner = owner_;
        treasury = treasury_;
        universalRouter = universalRouter_;
        permit2 = permit2_;
        hooksAllowed = hooksAllowed_;
    }

    modifier onlyOwner() {
        _onlyOwner();
        _;
    }

    modifier nonReentrant() {
        _nonReentrantBefore();
        _;
        _nonReentrantAfter();
    }

    receive() external payable {}

    function buyV4ExactEthForTokens(
        PoolKey calldata poolKey,
        address tokenOut,
        uint128 minTokensOut,
        uint256 deadline,
        bytes calldata hookData
    ) external payable onlyOwner nonReentrant returns (uint256 tokensReceived) {
        if (tokenOut == address(0)) revert ZeroAddress();
        if (msg.value == 0) revert ZeroAmount();
        _ensureDeadline(deadline);
        uint128 amountIn = _toUint128(msg.value);
        _ensureHookPolicy(poolKey, hookData);

        bool zeroForOne = _inferZeroForOne(poolKey, address(0), tokenOut);
        uint256 balanceBefore = IERC20(tokenOut).balanceOf(address(this));

        _executeV4Swap(poolKey, zeroForOne, address(0), tokenOut, amountIn, minTokensOut, deadline, hookData, msg.value);

        tokensReceived = IERC20(tokenOut).balanceOf(address(this)) - balanceBefore;
        if (tokensReceived < minTokensOut) revert InsufficientOutput(tokensReceived, minTokensOut);

        emit BoughtV4(tokenOut, msg.value, tokensReceived, minTokensOut, poolKey.hooks);
    }

    function emergencySellV4ExactTokensForEth(
        PoolKey calldata poolKey,
        address tokenIn,
        uint128 amountIn,
        uint128 minEthOut,
        uint256 deadline,
        bytes calldata hookData
    ) external onlyOwner nonReentrant returns (uint256 ethReceived) {
        if (tokenIn == address(0)) revert ZeroAddress();
        if (amountIn == 0) revert ZeroAmount();
        _ensureDeadline(deadline);
        _ensureHookPolicy(poolKey, hookData);

        bool zeroForOne = _inferZeroForOne(poolKey, tokenIn, address(0));
        uint256 balanceBefore = address(this).balance;

        tokenIn.safeApprove(permit2, 0);
        tokenIn.safeApprove(permit2, amountIn);
        IPermit2(permit2).approve(tokenIn, universalRouter, amountIn, PERMIT2_EXPIRATION);

        _executeV4Swap(poolKey, zeroForOne, tokenIn, address(0), amountIn, minEthOut, deadline, hookData, 0);

        IPermit2(permit2).approve(tokenIn, universalRouter, 0, 0);
        tokenIn.safeApprove(permit2, 0);

        ethReceived = address(this).balance - balanceBefore;
        if (ethReceived < minEthOut) revert InsufficientOutput(ethReceived, minEthOut);

        treasury.safeTransferEth(ethReceived);
        emit EmergencySoldV4(tokenIn, amountIn, ethReceived, minEthOut, poolKey.hooks);
    }

    function rescueToken(address token, address to, uint256 amount) external onlyOwner nonReentrant {
        if (token == address(0) || to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        token.safeTransfer(to, amount);
        emit RescueToken(token, to, amount);
    }

    function rescueEth(address to, uint256 amount) external onlyOwner nonReentrant {
        if (to == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        to.safeTransferEth(amount);
        emit RescueEth(to, amount);
    }

    function _executeV4Swap(
        PoolKey calldata poolKey,
        bool zeroForOne,
        address tokenIn,
        address tokenOut,
        uint128 amountIn,
        uint128 minAmountOut,
        uint256 deadline,
        bytes calldata hookData,
        uint256 value
    ) private {
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = _encodeV4SwapInput(poolKey, zeroForOne, tokenIn, tokenOut, amountIn, minAmountOut, hookData);
        IUniversalRouter(universalRouter).execute{value: value}(abi.encodePacked(COMMAND_V4_SWAP), inputs, deadline);
    }

    function _encodeV4SwapInput(
        PoolKey calldata poolKey,
        bool zeroForOne,
        address tokenIn,
        address tokenOut,
        uint128 amountIn,
        uint128 minAmountOut,
        bytes calldata hookData
    ) private view returns (bytes memory) {
        bytes memory actions = abi.encodePacked(
            bytes1(ACTION_SWAP_EXACT_IN_SINGLE), bytes1(ACTION_SETTLE), bytes1(ACTION_TAKE)
        );
        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(
            ExactInputSingleParams({
                poolKey: poolKey,
                zeroForOne: zeroForOne,
                amountIn: amountIn,
                amountOutMinimum: minAmountOut,
                hookData: hookData,
                minHopPriceX36: 0
            })
        );
        params[1] = abi.encode(tokenIn, amountIn, true);
        params[2] = abi.encode(tokenOut, address(this), uint256(0));
        return abi.encode(actions, params);
    }

    function _ensureDeadline(uint256 deadline) private view {
        if (block.timestamp > deadline) revert DeadlineExpired(deadline);
    }

    function _toUint128(uint256 amount) private pure returns (uint128) {
        if (amount > type(uint128).max) revert AmountTooLarge(amount);
        // Casting is safe because the range check above enforces uint128 max.
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint128(amount);
    }

    function _ensureHookPolicy(PoolKey calldata poolKey, bytes calldata hookData) private view {
        if (poolKey.currency0 == poolKey.currency1) revert InvalidPoolKey();
        if (poolKey.hooks == address(0)) {
            if (hookData.length != 0) revert HookDataWithoutHook();
            return;
        }
        if (!hooksAllowed) revert HookNotAllowed(poolKey.hooks);
    }

    function _inferZeroForOne(PoolKey calldata poolKey, address tokenIn, address tokenOut) private pure returns (bool) {
        if (tokenIn == poolKey.currency0 && tokenOut == poolKey.currency1) return true;
        if (tokenIn == poolKey.currency1 && tokenOut == poolKey.currency0) return false;
        revert InvalidPoolKey();
    }

    function _onlyOwner() private view {
        if (msg.sender != owner) revert Unauthorized();
    }

    function _nonReentrantBefore() private {
        if (_entered == 1) revert ReentrantCall();
        _entered = 1;
    }

    function _nonReentrantAfter() private {
        _entered = 0;
    }
}
