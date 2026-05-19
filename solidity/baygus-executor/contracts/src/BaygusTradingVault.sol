// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "./interfaces/IERC20.sol";
import {IUniswapV2Router02} from "./interfaces/IUniswapV2Router02.sol";
import {SafeTransferLib} from "./libraries/SafeTransferLib.sol";

contract BaygusTradingVault {
    using SafeTransferLib for address;

    error DeadlineExpired(uint256 deadline);
    error InsufficientOutput(uint256 actual, uint256 minimum);
    error ReentrantCall();
    error Unauthorized();
    error ZeroAddress();
    error ZeroAmount();

    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable owner;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable treasury;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable weth;
    // forge-lint: disable-next-line(screaming-snake-case-immutable)
    address public immutable uniswapV2Router;

    uint256 private _entered;

    event BoughtV2(address indexed token, uint256 ethIn, uint256 tokensReceived, uint256 minTokensOut);
    event EmergencySoldV2(address indexed token, uint256 tokenIn, uint256 ethReceived, uint256 minEthOut);
    event RescueEth(address indexed to, uint256 amount);
    event RescueToken(address indexed token, address indexed to, uint256 amount);

    constructor(address owner_, address treasury_, address weth_, address uniswapV2Router_) {
        if (owner_ == address(0) || treasury_ == address(0) || weth_ == address(0) || uniswapV2Router_ == address(0)) {
            revert ZeroAddress();
        }
        owner = owner_;
        treasury = treasury_;
        weth = weth_;
        uniswapV2Router = uniswapV2Router_;
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

    /// Mode A entry: buy through the vault and hold the token without approving
    /// the sell router. Approval is deferred until an emergency sell is needed.
    function buyV2ExactEthForTokens(address token, uint256 minTokensOut, uint256 deadline)
        external
        payable
        onlyOwner
        nonReentrant
        returns (uint256 tokensReceived)
    {
        if (token == address(0)) revert ZeroAddress();
        if (msg.value == 0) revert ZeroAmount();
        _ensureDeadline(deadline);

        uint256 balanceBefore = IERC20(token).balanceOf(address(this));
        address[] memory path = _ethToTokenPath(token);
        IUniswapV2Router02(uniswapV2Router)
        .swapExactETHForTokensSupportingFeeOnTransferTokens{
            value: msg.value
        }(minTokensOut, path, address(this), deadline);

        tokensReceived = IERC20(token).balanceOf(address(this)) - balanceBefore;
        if (tokensReceived < minTokensOut) revert InsufficientOutput(tokensReceived, minTokensOut);

        emit BoughtV2(token, msg.value, tokensReceived, minTokensOut);
    }

    /// Mode A emergency exit: approve the exact sell amount and swap in the same
    /// top-level transaction. No standing allowance is required before this call.
    function emergencySellV2ExactTokensForEth(address token, uint256 amountIn, uint256 minEthOut, uint256 deadline)
        external
        onlyOwner
        nonReentrant
        returns (uint256 ethReceived)
    {
        if (token == address(0)) revert ZeroAddress();
        if (amountIn == 0) revert ZeroAmount();
        _ensureDeadline(deadline);

        uint256 balanceBefore = address(this).balance;
        address[] memory path = _tokenToEthPath(token);

        token.safeApprove(uniswapV2Router, 0);
        token.safeApprove(uniswapV2Router, amountIn);
        IUniswapV2Router02(uniswapV2Router)
            .swapExactTokensForETHSupportingFeeOnTransferTokens(amountIn, minEthOut, path, address(this), deadline);
        token.safeApprove(uniswapV2Router, 0);

        ethReceived = address(this).balance - balanceBefore;
        if (ethReceived < minEthOut) revert InsufficientOutput(ethReceived, minEthOut);

        treasury.safeTransferEth(ethReceived);
        emit EmergencySoldV2(token, amountIn, ethReceived, minEthOut);
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

    function _ensureDeadline(uint256 deadline) private view {
        if (block.timestamp > deadline) revert DeadlineExpired(deadline);
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

    function _ethToTokenPath(address token) private view returns (address[] memory path) {
        path = new address[](2);
        path[0] = weth;
        path[1] = token;
    }

    function _tokenToEthPath(address token) private view returns (address[] memory path) {
        path = new address[](2);
        path[0] = token;
        path[1] = weth;
    }
}
