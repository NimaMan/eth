# Flashbots Integration for ETH Kartal 🤖⚡

## Overview

Flashbots is a **private transaction pool** and **block building** infrastructure that allows traders to submit transactions directly to miners/validators without exposing them to the public mempool. This is critical for MEV protection and ensuring transaction execution in competitive scenarios.

## Why We Need Flashbots

### 1. **Front-Running Protection** 🛡️
When we detect a scam and need to sell tokens immediately, broadcasting to the public mempool exposes our transaction to:
- **MEV Bots**: That can see our transaction and front-run it
- **Sandwich Attacks**: Bots that insert transactions before and after ours
- **Competition**: Other traders racing for the same opportunity

### 2. **Guaranteed Inclusion** ✅
For **Critical** priority alerts, we need certainty that our protective transaction will be included:
- **Bundle Atomicity**: All-or-nothing execution
- **Direct to Validator**: Bypasses mempool congestion
- **Priority Ordering**: Pay for specific position in block

### 3. **Failed Transaction Protection** 💸
Public mempool transactions that fail still cost gas. Flashbots bundles:
- **Only pay if included**: No gas cost for failed/rejected bundles
- **Simulation first**: Pre-validate bundle execution
- **Revert protection**: Bundle discarded if any transaction reverts

### 4. **Speed in Critical Moments** ⚡
When protecting users from scams, every millisecond counts:
- **Skip mempool propagation**: Direct validator submission
- **No gas price wars**: Fixed priority fee negotiation
- **Predictable inclusion**: Know exactly which block

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────┐
│ Alert       │────▶│ Bundle       │────▶│ Flashbots   │────▶│ Validator│
│ (Critical)  │     │ Builder      │     │ Relay       │     │ (Block)  │
└─────────────┘     └──────────────┘     └─────────────┘     └──────────┘
                           │                      │
                           ▼                      ▼
                    ┌──────────────┐      ┌─────────────┐
                    │ Sign Bundle  │      │ Simulate    │
                    │ + Add Tip    │      │ Bundle      │
                    └──────────────┘      └─────────────┘
```

### Bundle Structure
A Flashbots bundle consists of:
1. **Signed Transactions**: One or more transactions to execute atomically
2. **Target Block**: Specific block number for inclusion
3. **Min/Max Timestamp**: Time validity window
4. **Revert Protection**: Flag to discard bundle if any tx reverts

### Execution Flow

1. **Alert Received** (Priority: Critical)
   ```rust
   Alert { 
       action: Sell, 
       priority: Critical,
       token: "0xSCAM..." 
   }
   ```

2. **Build Bundle**
   ```rust
   Bundle {
       transactions: vec![sell_tx],
       block_number: current_block + 1,
       min_timestamp: now,
       max_timestamp: now + 12 seconds,
   }
   ```

3. **Submit to Relay**
   - Sign bundle with our private key
   - Add priority fee (tip to validator)
   - Submit to multiple relays for redundancy

4. **Monitor Inclusion**
   - Check if bundle was included
   - Retry with higher tip if needed
   - Fallback to public mempool if necessary

## Implementation Architecture

### Core Components

#### 1. **Bundle Builder** (`bundle.rs`)
- Constructs valid Flashbots bundles
- Handles transaction ordering
- Calculates optimal tips

#### 2. **Relay Client** (`client.rs`)
- Submits bundles to Flashbots relays
- Handles authentication (signature)
- Manages retries and fallbacks

#### 3. **Simulation Engine** (`simulation.rs`)
- Pre-simulates bundle execution
- Validates bundle will succeed
- Estimates gas and profits

#### 4. **Bundle Signer** (`signer.rs`)
- Signs bundles with reputation key
- Manages Flashbots reputation
- Handles signature schemas

## Configuration

```rust
pub struct FlashbotsConfig {
    /// Relay endpoints (mainnet)
    pub relay_endpoints: Vec<String>,
    
    /// Reputation private key (for signing bundles)
    pub reputation_key: SecretKey,
    
    /// Maximum tip percentage (of transaction value)
    pub max_tip_percentage: f64,
    
    /// Bundle submission timeout
    pub submission_timeout: Duration,
    
    /// Enable bundle simulation
    pub simulate_before_submit: bool,
}
```

## When to Use Flashbots

### ✅ **Always Use For:**
- **Critical** priority alerts (scam protection)
- High-value transactions (>$10k)
- Time-sensitive protective actions
- Known MEV-vulnerable operations

### ⚠️ **Consider For:**
- **High** priority alerts
- Moderate value transactions ($1k-$10k)
- Competitive token launches
- Volatile market conditions

### ❌ **Don't Use For:**
- **Normal** priority alerts
- Low-value transactions (<$1k)
- Non-time-sensitive operations
- Testing/development

## Security Considerations

1. **Bundle Privacy**: Bundles are not public until included
2. **Reputation System**: Bad bundles hurt reputation score
3. **Signature Verification**: All bundles must be signed
4. **Relay Trust**: Use multiple reputable relays
5. **Fallback Strategy**: Always have public mempool backup

## Performance Metrics

Target performance for Flashbots integration:
- **Bundle Build Time**: <5ms
- **Relay Submission**: <50ms
- **Total Overhead**: <100ms
- **Success Rate**: >95% for critical alerts

## Example Usage

```rust
// Critical alert requires Flashbots
if alert.priority == Priority::Critical {
    // Build bundle
    let bundle = flashbots.build_bundle(vec![swap_tx])
        .block_number(current_block + 1)
        .tip_percentage(0.01) // 1% tip
        .revert_protection(true)
        .build()?;
    
    // Submit to relays
    let result = flashbots.submit_bundle(bundle).await?;
    
    match result {
        BundleResult::Included(block) => {
            info!("Bundle included in block {}", block);
        }
        BundleResult::NotIncluded => {
            warn!("Bundle not included, falling back to mempool");
            // Fallback to public submission
        }
    }
}
```

## Relay Endpoints

### Mainnet Relays
- **Flashbots**: `https://relay.flashbots.net`
- **BloXroute**: `https://mev.api.blxrbdn.com`
- **Eden**: `https://api.edennetwork.io/v1/bundle`
- **Manifold**: `https://api.manifold.finance/v1/bundle`

### Monitoring & Analytics
- Bundle status: `https://protect.flashbots.net`
- MEV-Boost stats: `https://mevboost.org`

## Benefits Summary

1. **Protection**: No front-running or sandwich attacks
2. **Certainty**: Guaranteed atomic execution
3. **Efficiency**: Only pay for successful inclusion
4. **Speed**: Direct validator submission
5. **Privacy**: Transactions hidden until included

Flashbots is essential for eth_kartal's mission of protecting users from scams with the fastest, most reliable execution possible.