// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IPermit2} from "../../src/interfaces/IPermit2.sol";
import {MockERC20} from "./MockERC20.sol";

contract MockPermit2 is IPermit2 {
    bytes32 public constant VALID_SIGNATURE_HASH = keccak256("MockPermit2: valid signature");

    struct Allowance {
        uint160 amount;
        uint48 expiration;
    }

    mapping(address => mapping(address => mapping(address => Allowance))) public allowance;
    mapping(address => mapping(uint256 => uint256)) public nonceBitmap;

    event Approval(
        address indexed owner, address indexed token, address indexed spender, uint160 amount, uint48 expiration
    );
    event TransferFrom(address indexed caller, address indexed from, address indexed to, address token, uint160 amount);
    event SignatureTransfer(
        address indexed caller,
        address indexed owner,
        address indexed to,
        address token,
        uint256 permittedAmount,
        uint256 nonce,
        uint256 requestedAmount
    );

    function approve(address token, address spender, uint160 amount, uint48 expiration) external {
        allowance[msg.sender][token][spender] = Allowance({amount: amount, expiration: expiration});
        emit Approval(msg.sender, token, spender, amount, expiration);
    }

    function transferFrom(address from, address to, uint160 amount, address token) external override {
        Allowance storage permitted = allowance[from][token][msg.sender];
        require(permitted.expiration >= block.timestamp, "MockPermit2: expired");
        require(permitted.amount >= amount, "MockPermit2: allowance");
        if (permitted.amount != type(uint160).max) {
            permitted.amount -= amount;
        }
        require(MockERC20(token).transferFrom(from, to, amount), "MockPermit2: transfer");
        emit TransferFrom(msg.sender, from, to, token, amount);
    }

    function permitTransferFrom(
        PermitTransferFrom memory permit,
        SignatureTransferDetails calldata transferDetails,
        address owner,
        bytes calldata signature
    ) external override {
        require(keccak256(signature) == VALID_SIGNATURE_HASH, "MockPermit2: signature");
        _permitTransferFrom(permit, transferDetails, owner);
    }

    function permitWitnessTransferFrom(
        PermitTransferFrom memory permit,
        SignatureTransferDetails calldata transferDetails,
        address owner,
        bytes32 witness,
        string calldata witnessTypeString,
        bytes calldata signature
    ) external override {
        require(
            keccak256(signature)
                == keccak256(abi.encodePacked("MockPermit2: valid witness signature", witness, witnessTypeString)),
            "MockPermit2: signature"
        );
        _permitTransferFrom(permit, transferDetails, owner);
    }

    function _permitTransferFrom(
        PermitTransferFrom memory permit,
        SignatureTransferDetails calldata transferDetails,
        address owner
    ) internal {
        require(block.timestamp <= permit.deadline, "MockPermit2: deadline");
        require(transferDetails.requestedAmount <= permit.permitted.amount, "MockPermit2: amount");

        (uint256 wordPos, uint256 bitPos) = _bitmapPositions(permit.nonce);
        uint256 bit = uint256(1) << bitPos;
        require(nonceBitmap[owner][wordPos] & bit == 0, "MockPermit2: nonce");
        nonceBitmap[owner][wordPos] |= bit;

        require(
            MockERC20(permit.permitted.token).transferFrom(owner, transferDetails.to, transferDetails.requestedAmount),
            "MockPermit2: transfer"
        );
        emit SignatureTransfer(
            msg.sender,
            owner,
            transferDetails.to,
            permit.permitted.token,
            permit.permitted.amount,
            permit.nonce,
            transferDetails.requestedAmount
        );
    }

    function _bitmapPositions(uint256 nonce) internal pure returns (uint256 wordPos, uint256 bitPos) {
        wordPos = nonce >> 8;
        bitPos = nonce & 0xff;
    }
}
