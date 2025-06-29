# ETH Kartal Simulation Examples

This directory contains comprehensive simulation tools for testing ETH Kartal's capabilities without executing real transactions. All simulations use the secure wallet implementation and connect to your local Reth node.

## 🎯 Purpose

These simulators allow you to:
- Test trading strategies without risking real funds
- Verify transaction parameters and gas costs
- Practice emergency scenarios
- Validate the secure wallet implementation
- Analyze arbitrage opportunities
- Optimize gas usage strategies

## 🛠️ Available Simulators

### 1. **Simple Swap Simulator** (`simple_swap_simulator.rs`)
The most basic simulator - demonstrates ETH to USDC swaps with clear output.

**What it does:**
- Simulates swapping 0.05 ETH to USDC
- Shows expected output amounts
- Estimates gas costs
- Calculates slippage protection
- NO real transactions sent

**Run it:**
```bash
cargo run --example simple_swap_simulator
```

### 2. **Swap Simulator** (`swap_simulator.rs`)
Interactive swap simulator with multiple options and custom amounts.

**What it does:**
- Interactive menu for different swap pairs
- Supports ETH→USDC, ETH→USDT, USDC→ETH
- Custom swap amounts
- Real-time pool selection
- Detailed gas analysis

**Run it:**
```bash
cargo run --example swap_simulator
```

### 3. **Secure Wallet Swap Simulator** (`secure_wallet_swap_simulator.rs`)
Advanced simulator using the encrypted keystore wallet.

**What it does:**
- Uses your encrypted keystore (not plain text private key)
- Requires password to unlock
- Simulates various swap scenarios
- Emergency sell simulation
- Arbitrage opportunity detection

**Setup required:**
```bash
# First create a keystore from KARTAL_KILIT
cargo run --bin keystore_manager -- create

# Then run the simulator
cargo run --example secure_wallet_swap_simulator
```

**Features:**
- 0.05 ETH → USDC swap simulation
- 0.1 ETH → USDT swap simulation  
- Emergency sell (all ETH → USDC)
- Three-hop arbitrage simulation (ETH→USDC→USDT→ETH)

### 4. **Transfer Simulator** (`transfer_simulator.rs`)
Simulates ETH and token transfers with gas optimization analysis.

**What it does:**
- ETH transfer simulation with gas estimation
- ERC20 token transfer simulation
- Batch transfer analysis (multiple recipients)
- Gas optimization strategies
- Network condition analysis

**Run it:**
```bash
cargo run --example transfer_simulator
```

**Features:**
- Real balance checking
- Insufficient balance detection
- Gas price comparison (slow/normal/fast/instant)
- Best times to transfer analysis

### 5. **Portfolio Simulator** (`portfolio_simulator.rs`)
Complete portfolio management simulation with risk controls.

**What it does:**
- Virtual portfolio tracking
- Buy/sell signal simulation
- Portfolio rebalancing
- Risk limit testing
- P&L calculation

**Run it:**
```bash
cargo run --example portfolio_simulator
```

**Starting portfolio:**
- 5 ETH
- 10,000 USDC
- 5,000 USDT

**Features:**
- Simulate buy signals (e.g., SHIB purchase)
- Emergency sell simulation
- Portfolio rebalancing to target allocations
- Risk limit scenarios (position size, loss limits, etc.)

## 🔐 Security Features

All simulators incorporate our secure wallet implementation:
- **Encrypted keystores** - No plain text private keys
- **Password protection** - Wallet unlocking required
- **Automatic locking** - Wallet locks after use
- **No real transactions** - Everything is simulated

## 📊 What Gets Simulated

### Transaction Details
- Input/output amounts
- Exchange rates
- Gas prices and limits
- Total transaction costs
- Slippage calculations

### Market Conditions  
- Current gas prices from your Reth node
- Pool liquidity checks
- Price impact analysis
- Network congestion estimates

### Risk Factors
- Insufficient balance scenarios
- High slippage warnings
- Gas spike protection
- Failed transaction handling

## 🚀 Quick Start

1. **Ensure Reth node is running:**
```bash
ps aux | grep reth
```

2. **Check KARTAL_KILIT environment variable:**
```bash
echo $KARTAL_KILIT
```

3. **Create secure keystore (if needed):**
```bash
cargo run --bin keystore_manager -- create
```

4. **Run any simulator:**
```bash
# Simple demo
cargo run --example simple_swap_simulator

# Interactive with secure wallet
cargo run --example secure_wallet_swap_simulator
```

## 🧪 Test Mode vs Simulation

### Simulators (This Directory)
- Standalone programs for testing specific scenarios
- No alert processing
- Direct function calls
- Perfect for development and testing

### Test Mode (`--test-mode` flag)
- Full system running in simulation mode
- Processes real alerts without executing
- Complete execution flow testing
- Use for system integration testing

```bash
# Run full system in test mode
cargo run --bin kartal -- --test-mode
```

## 📈 Example Output

```
=== Simple ETH → USDC Swap Simulator ===

Simulating swap of 0.05 ETH to USDC...

Current block: 18976543

📊 Swap Simulation Results:
═══════════════════════════
Input:    0.05 ETH
Output:   ~125.00 USDC
Pool:     Uniswap V2 WETH/USDC
Address:  0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc

⛽ Gas Estimation:
══════════════════
Gas Price: 15.23 gwei
Gas Limit: 200000
Total Cost: 0.003046 ETH

✅ SIMULATION COMPLETE
══════════════════════
This was a simulation only - no transaction was sent!
No gas was spent, no funds were moved.
```

## 🛡️ Safety Guarantees

1. **No real transactions** - All simulators only read from the blockchain
2. **No wallet modifications** - Your funds remain untouched
3. **No approvals needed** - No token permissions required
4. **Local node only** - Uses your local Reth node (127.0.0.1:8545)

## 🔧 Customization

To modify simulation parameters, edit the constants in each file:
- Token addresses
- Swap amounts  
- Slippage tolerances
- Gas estimates

## 📝 Best Practices

1. Always run simulations before implementing new strategies
2. Test edge cases (insufficient balance, high slippage, etc.)
3. Verify gas estimates match current network conditions
4. Use secure wallet simulator for production-like testing
5. Compare simulation results with actual transactions

## 🐛 Troubleshooting

**"Keystore not found"**
- Run `cargo run --bin keystore_manager -- create`

**"Failed to connect to node"**
- Ensure Reth is running on 127.0.0.1:8545

**"Insufficient balance"**
- Normal in simulation - shows the check is working

**Compilation errors**
- Run `cargo check` to see detailed errors
- Ensure all dependencies are up to date

## 🚨 Important Notes

- These are **SIMULATORS ONLY** - no real transactions
- Always verify addresses before using in production
- Gas estimates may vary from actual execution
- Pool states change rapidly - results are snapshots

Use these simulators to safely explore ETH Kartal's capabilities!