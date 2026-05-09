// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface Vm {
    function coinbase(address newCoinbase) external;
    function deal(address account, uint256 newBalance) external;
    function prank(address msgSender) external;
    function roll(uint256 newBlockNumber) external;
}

contract TestBase {
    // forge-lint: disable-next-line(screaming-snake-case-const)
    Vm internal constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    error AssertionFailed(string message);

    function assertEq(uint256 a, uint256 b, string memory message) internal pure {
        if (a != b) revert AssertionFailed(message);
    }

    function assertEq(address a, address b, string memory message) internal pure {
        if (a != b) revert AssertionFailed(message);
    }

    function assertEq(int128 a, int128 b, string memory message) internal pure {
        if (a != b) revert AssertionFailed(message);
    }

    function assertEq(bytes memory a, bytes memory b, string memory message) internal pure {
        if (keccak256(a) != keccak256(b)) revert AssertionFailed(message);
    }

    function assertTrue(bool condition, string memory message) internal pure {
        if (!condition) revert AssertionFailed(message);
    }

    function assertFalse(bool condition, string memory message) internal pure {
        if (condition) revert AssertionFailed(message);
    }

    receive() external payable {}
}
