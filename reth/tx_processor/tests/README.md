# TX Processor Tests

This directory contains tests for the tx_processor module.

## Test Files

### event_decoder_tests.rs
Unit tests for the event decoder functionality:
- ERC20 transfer decoding
- ERC721 transfer decoding
- Uniswap event decoding
- Other DeFi protocol events

Run with: `cargo test --test event_decoder_tests`

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test event_decoder_tests

# Run with output
cargo test -- --nocapture
```

## Requirements

- Synced Reth database at the path specified by `RETH_DATADIR` environment variable
- Default path: `/home/nima/.local/share/reth/mainnet`