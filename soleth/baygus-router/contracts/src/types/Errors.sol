// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

error AdapterMissing(uint8 command);
error CommandLengthMismatch();
error EmptyPath();
error InvalidCommand(uint8 command);
error InvalidTransferFromInput();
error MissingPoolManager();
error NativeSettleFailed();
error ReentrantCall();
error SlippageCheckFailed(uint8 index, int128 actual, int128 minimum);
error SweepInsufficientBalance(address token, uint256 balance, uint256 minimum);
error UnauthorizedCallback();
error V2SwapFailed();
error V3SwapFailed();
error CurveSwapFailed();
