# Wallet Examples

This folder contains examples demonstrating wallet usage and swap simulations using KARTAL_KILIT.

## Example

### `swap_sim_with_kartal_kilit.rs`

**Purpose**: Real-time swap simulation using KARTAL_KILIT private key

**Input**: 
- `KARTAL_KILIT` environment variable (your private key)
- Local Ethereum node at `http://127.0.0.1:8545`

**Output**:
- Wallet address and ETH balance
- Real-time swap quotes:
  - 0.05 ETH → USDC conversion rate and output
  - 0.1 ETH → USDT conversion rate and output
- Current gas prices and block information
- Slippage analysis for different percentages

**Value**: 
- Uses your actual wallet (no keystore needed)
- Provides real mainnet data
- Shows current exchange rates
- Ready for actual transaction execution

## Running the Example

```bash
# Ensure KARTAL_KILIT is set
export KARTAL_KILIT=your_private_key_here

# Run the simulation
cargo run --example swap_sim_with_kartal_kilit
```

## Sample Output

```
=== Swap Simulator with KARTAL_KILIT ===

Wallet address: 0xb340…245c
Current ETH Balance: 0.0736 ETH

🔄 Simulating: 0.05 ETH → USDC
Best pool: 0xb4e1…c9dc (UniswapV2)
Exchange rate: 1 ETH = 3547.52 USDC
Expected output: 177.38 USDC

⛽ Gas Analysis:
Gas price: 0.797 gwei
Total gas cost: 0.000159 ETH
```

This example demonstrates how to use KARTAL_KILIT directly for swap simulations without the complexity of keystore management.

## Note on Keystore Support

While this example uses KARTAL_KILIT directly for simplicity, eth_kartal also supports standard Ethereum keystore files for secure key storage. Keystore support is included as a future option for users who need:
- Encrypted key storage at rest
- Password-protected access
- Compliance with enterprise security policies

For production deployments requiring keystore functionality, the `eth-keystore` crate is already included as a dependency and can be utilized through the SecureWallet implementation.