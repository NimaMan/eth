# Baygus Executor Mainnet Validation

This folder is for pre-deployment Baygus Executor validation against real Ethereum mainnet state.
Keep these examples focused on the executor surface: deploy the current Soleth bytecode into
the local simulation state, compose Baygus commands with `tx_simulator::tx_builders`, execute the
sequence, and assert balances, traces, gas, and revert behavior.

Validation is also a pruning tool. These examples should show which parts of the executor are worth
deploying and which parts are better handled off-chain or removed from production bytecode. The
default answer for live deployment is a smaller executor; the full command surface must earn its gas
with benchmarked value.

## Why stablecoin pools first

Stablecoin routes are the right first target because they exercise the executor without adding price
volatility noise:

- USDC and USDT use 6 decimals; DAI and FRAX use 18 decimals.
- USDT is a useful non-standard token edge case for transfers and approvals.
- Stable-to-stable routes test token-to-token command execution without native ETH wrapping.
- WETH/stable routes test the normal execution path that the live executor will use for buys/sells.
- Curve 3pool and FRAX/USDC add non-Uniswap command coverage.

## Initial Pool Matrix

Pool targets were checked against local Reth RPC at block `25028188`.

| Tier | Protocol | Route | Fee | Pool | Why |
| --- | --- | --- | --- | --- | --- |
| P0 | Uniswap V2 | WETH/USDC | 30 bps | `0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc` | canonical ETH-funded buy/sell path |
| P0 | Uniswap V2 | USDC/USDT | 30 bps | `0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f` | 6-decimal stable-to-stable token path |
| P0 | Uniswap V3 | USDC/USDT | 1 bp | `0x3416cF6C708Da44DB2624D63ea0AAef7113527C6` | deep stable pool and exactInputSingle path |
| P0 | Uniswap V3 | USDC/DAI | 1 bp | `0x5777d92f208679DB4b9778590Fa3CAB3aC9e2168` | 6-decimal to 18-decimal stable path |
| P0 | Curve V1 | DAI/USDC/USDT 3pool | n/a | `0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7` | stable-native Curve command path |
| P1 | Sushi V2 | WETH/USDC | 30 bps | `0x397FF1542f962076d0BFE58eA045FfA2d347ACa0` | alternate V2 adapter |
| P1 | Uniswap V2 | WETH/USDT | 30 bps | `0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852` | USDT sell/approval behavior with WETH |
| P1 | Uniswap V3 | WETH/USDC | 5 bps | `0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640` | high-volume WETH/stable V3 path |
| P1 | Curve V1 | FRAX/USDC | n/a | `0xDcEF968d416a41Cdac0ED8702fAC8128A64241A2` | 18-decimal FRAX to 6-decimal USDC |
| P1 | Uniswap V4 | WETH/USDC | 4.9 bps | PoolManager `0x000000000004444C5DC75cB358380d2E3de08a90`, pool id `0x11142dd4ac627021305b9349c2167d89744c4e45c92ce383c04120337f86495c` | configured v4 WETH/stable pool; latest-state validation currently returns zero output |
| P1 | Uniswap V4 | WETH/USDC | 4.5 bps | PoolManager `0x000000000004444C5DC75cB358380d2E3de08a90`, pool id `0xd0bd244aa79359d02de8a457c97944fa34b1d7203c8cf7aac1e25cefbe6ead4f` | no-hook v4 WETH/stable pool discovered from Initialize logs; latest-state validation currently returns zero output |
| P2 | Uniswap V3 | DAI/USDT | 1 bp | `0x48DA0965ab2d2cbf1C17C09cFB5Cbe67Ad5B1406` | 18-decimal to 6-decimal stable path |
| P2 | Uniswap V2 | USDC/DAI | 30 bps | `0xAE461cA67B15dc8dc81CE7615e0320dA1A9aB8D5` | V2 stable pair with mixed decimals |
| P2 | Uniswap V3 | LUSD/USDC | 5 bps | `0x4e0924d3a751bE199C426d52fb1f2337fa96f736` | smaller stablecoin path for failure/slippage sensitivity |

## Test Shape

Each executable validation should run the same pattern:

1. Build a simulation chain at a fixed recent block.
2. Deploy the current `soleth/baygus-executor/out/BaygusExecutor.sol/BaygusExecutor.json` bytecode into
   the simulated state with mainnet adapter addresses.
3. Fund a deterministic test account with ETH and/or token balances using state overrides or setup
   transfers from known rich accounts.
4. Build the Baygus command plan from typed Rust builders.
5. Simulate the sequence with traces enabled.
6. Assert:
   - transaction success;
   - expected token balance deltas;
   - executor has no stranded token/native balance unless the test explicitly expects it;
   - coinbase tip paid only when block bounds match;
   - gas is recorded and below the configured ceiling;
   - trace contains the expected external adapter calls.

## First Executable Targets

Build these first:

- `baygus_eth_stable_quotes` / `baygus_eth_stable_quotes.rs`: deploy the current Baygus bytecode into
  the forked local Reth state, approve WETH once, wrap exactly `1 ETH` per route, execute all
  configured ETH-funded stable routes (Uniswap V2, SushiSwap V2, Uniswap V3, and investigated
  Uniswap V4 WETH/USDC pools), and assert Baygus output equals the direct V2/V3 quote where a
  router or quoter quote exists. V4 pools are reported as investigation findings when they execute
  but produce zero output at the latest local block.
- `baygus_gas_benchmark.rs`: compares direct router execution gas against Baygus Executor command
  execution gas for representative WETH/stable routes. It reports both executor pull modes
  (`transfer_from` and Permit2), and prints setup/deploy gas separately.
- `v2_stable_execute_plan.rs`: `transfer_from -> v2_swap -> sweep`, plus optional guarded
  `coinbase_tip`.
- `v3_stable_execute_plan.rs`: `transfer_from -> v3_swap -> sweep`, plus optional guarded
  `coinbase_tip`.
- `curve_stable_execute_plan.rs`: `transfer_from -> curve_swap -> sweep`.
- `executor_deploy_smoke.rs`: deploy only, assert bytecode exists, constructor adapters are readable.

After these pass, add Balancer pool-id based tests and then mainnet deployment dry-run output.

When a Baygus route is materially more expensive than the direct path, prefer one of these fixes
before deploying:

- move routing or validation logic off-chain;
- send final swap output directly to the recipient instead of sweeping;
- use direct pair/pool execution instead of a generic router adapter;
- split a minimal production executor from the broader research executor;
- drop commands that are not required by the current strategy.

Run the ETH/stable quote check with:

```sh
cargo run -p tx_simulator --example baygus_eth_stable_quotes
cargo run -p tx_simulator --example baygus_gas_benchmark
```

Useful environment flags:

- `BAYGUS_SIM_BLOCK=<block>` pins the simulation state instead of using the latest local block.
- `BAYGUS_TRACE_FAILURE=1` prints the call trace for a failed route.
- `BAYGUS_TRACE_SUCCESS=1` prints the call trace for successful routes.

For V4 latest-state zero-output findings, rerun at the route's printed `BAYGUS_SIM_BLOCK` hint to
separate an executor issue from a pool-state/liquidity issue at the latest block.
At block `23560197`, the 4.9bp WETH/USDC V4 route currently produces nonzero USDC through
`BaygusExecutor`, so the latest-block zero-output finding is tracked as a pool-state investigation
rather than a known executor revert.
