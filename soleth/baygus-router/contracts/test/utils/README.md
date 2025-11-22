# Test Utilities

## Problem Domain
This directory contains helper contracts and shared setup logic for the test suite. It reduces code duplication across test files and ensures a consistent testing environment.

## Logic
*   **TestBase.sol**: Acts as the parent contract for test suites. It likely handles the `setUp()` function to deploy the `BaygusRouter`, mint mock tokens, and configure initial state (e.g., labels, initial balances) required before running specific test cases.
