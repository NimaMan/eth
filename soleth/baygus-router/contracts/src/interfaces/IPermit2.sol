// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title IPermit2
/// @notice Interface for Uniswap Permit2 contract
interface IPermit2 {
    struct PermitSingle {
        address details_token;
        uint256 details_amount;
        uint256 details_expiration;
        uint256 details_nonce;
    }

    struct PermitBatch {
        PermitSingle[] details;
    }

    struct SignatureTransferDetails {
        address token;
        uint256 amount;
        uint256 expiration;
        uint256 nonce;
    }

    function permitTransferFrom(
        SignatureTransferDetails memory transferDetails,
        address owner,
        bytes calldata signature,
        uint256 value
    ) external;

    function permitTransferFrom(
        PermitBatch memory permitBatch,
        address owner,
        bytes calldata signature,
        uint256 value
    ) external;

    function permit(address owner, PermitSingle calldata permitSingle, bytes calldata signature) external;

    function allowance(address owner, address token, address spender) external view returns (uint256 amount, uint256 expiration, uint256 nonce);
}
