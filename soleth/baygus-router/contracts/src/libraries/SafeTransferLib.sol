// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

library SafeTransferLib {
    error ApproveFailed();
    error EthTransferFailed();
    error TransferFailed();
    error TransferFromFailed();

    function safeTransferEth(address to, uint256 amount) internal {
        bool success;
        assembly {
            success := call(gas(), to, amount, 0, 0, 0, 0)
        }
        if (!success) revert EthTransferFailed();
    }

    function safeTransferFrom(address token, address from, address to, uint256 amount) internal {
        bool success;
        assembly {
            let ptr := mload(0x40)
            mstore(ptr, 0x23b872dd00000000000000000000000000000000000000000000000000000000)
            mstore(add(ptr, 4), from)
            mstore(add(ptr, 36), to)
            mstore(add(ptr, 68), amount)
            success := and(
                or(and(eq(mload(0), 1), gt(returndatasize(), 31)), iszero(returndatasize())),
                call(gas(), token, 0, ptr, 100, 0, 32)
            )
        }
        if (!success) revert TransferFromFailed();
    }

    function safeTransfer(address token, address to, uint256 amount) internal {
        bool success;
        assembly {
            let ptr := mload(0x40)
            mstore(ptr, 0xa9059cbb00000000000000000000000000000000000000000000000000000000)
            mstore(add(ptr, 4), to)
            mstore(add(ptr, 36), amount)
            success := and(
                or(and(eq(mload(0), 1), gt(returndatasize(), 31)), iszero(returndatasize())),
                call(gas(), token, 0, ptr, 68, 0, 32)
            )
        }
        if (!success) revert TransferFailed();
    }

    function safeApprove(address token, address spender, uint256 amount) internal {
        bool success;
        assembly {
            let ptr := mload(0x40)
            mstore(ptr, 0x095ea7b300000000000000000000000000000000000000000000000000000000)
            mstore(add(ptr, 4), spender)
            mstore(add(ptr, 36), amount)
            success := and(
                or(and(eq(mload(0), 1), gt(returndatasize(), 31)), iszero(returndatasize())),
                call(gas(), token, 0, ptr, 68, 0, 32)
            )
        }
        if (!success) revert ApproveFailed();
    }
}
