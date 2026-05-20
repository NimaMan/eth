# Uniswap V4 Trading Vault Assessment Criteria

These criteria define when the V4 vault is ready for mainnet deployment. They
are intentionally stricter than the V2 criteria because V4 execution depends on
Universal Router command encoding, Permit2, PoolManager settlement, hook policy,
and exact `PoolKey` semantics.

## Decision Rule

The V4 vault is deployable only when every required criterion is either:

- `passed`: evidence exists and no unresolved blocker remains;
- `accepted-risk`: a named operator and reviewer have accepted the residual
  risk in the run folder signoff.

Any `todo`, `partial`, `blocked`, or missing evidence means no mainnet
broadcast.

## Shared Criteria From V2

| Criterion | Required Evidence |
| --- | --- |
| Source identity | Git revision, dirty-state summary, source hash, artifact hash, bytecode hash, deployed-bytecode hash, and ABI hash. |
| Constructor args | Owner, treasury, chain id, deployed periphery addresses, signer, and constructor ABI encoding in the run folder. |
| Owner model | Unit tests and post-deploy reads prove only the intended owner can buy, sell, and rescue. |
| Treasury model | Tests and post-deploy reads prove ETH proceeds go to the intended treasury. |
| Allowance lifecycle | Tests prove no standing unsafe allowance remains after successful paths and failures revert allowance state. |
| Gas benchmark | Local and pinned fork gas reports compare vault route costs against direct execution. |
| Calldata evidence | Representative buy and emergency-sell calldata are generated from the same ABI used by tx prep. |
| Kartal policy | Target, selector, from address, value cap, gas cap, fee cap, simulation freshness, and daily cap are recorded. |
| Dry-run evidence | Kartal dry-run evidence is stored in the run folder before live broadcast is enabled. |
| Deploy fail-closed | Deploy script refuses to broadcast unless `CONFIRM_DEPLOY=1` and signer inputs are explicit. |
| Verification and smoke | Etherscan verification output and immutable readback are stored after deploy. |
| Signoff | Operator and reviewer signoff is recorded before public broadcast or live trading enablement. |

## V4-Specific Criteria

| Criterion | Required Evidence |
| --- | --- |
| Interface pinning | Universal Router, Permit2, PoolManager, and hook-related ABI assumptions are pinned with source/version/hash notes. |
| Exact PoolKey policy | Route fixture records `currency0`, `currency1`, `fee`, `tickSpacing`, `hooks`, token orientation, and expected hook data. |
| Universal Router command encoding | Tests or simulator reports prove command `0x10` and V4 actions `SWAP_EXACT_IN_SINGLE`, `SETTLE`, and `TAKE` are encoded in the expected order. |
| PoolManager settlement semantics | Simulator/fork evidence proves native ETH and ERC20 settlement succeeds through the deployed V4 periphery at the pinned block. |
| Hook policy | Default deployment rejects nonzero hooks; any hook-enabled deployment requires a separate fixture, separate policy, and explicit signoff. |
| Permit2 exact lifecycle | Sell path grants exact ERC20 allowance to Permit2 and exact Permit2 allowance to Universal Router, then clears both after success. |
| Recipient policy | Buy output token stays in the vault; emergency-sell native ETH is paid directly to treasury. |
| Route scope | Candidate supports only audited exact-input single-hop route shape. Dynamic path discovery, unknown hooks, and generic router calldata are out of scope. |
| Current-state simulation freshness | The final Kartal request must reference a simulation block within the configured freshness window. |
| V4 target allowlist | Kartal and signer policies must allow only the deployed V4 vault target and V4 trading selectors. Rescue selectors are excluded from live trading policy. |
| Nonce-bound address check | If deployment uses a predicted address, the deployer nonce must be rechecked immediately before broadcast and calldata regenerated if it changed. |
| Bribe policy | Only EIP-1559 priority fee is allowed. Direct coinbase transfers and private bundle payments require a separate protocol and are not part of this vault. |

## Current Candidate Evidence

Current gate run:
`runs/20260519-v4-gates-202032Z/`.

Evidence already present:

- source/artifact hashes: `artifact-hashes.json`;
- constructor args and init-code hash: `constructor-args.json` and
  `calldata.json`;
- pinned fork and tx_simulator comparison: `fork-rehearsal.json`;
- representative buy and emergency-sell calldata: `calldata.json`;
- Kartal policy-rejection dry-run against current V2 allowlist:
  `kartal-dry-run.json`;
- deploy fail-closed proof: `deploy-fail-closed.txt`.

Current blockers:

- upstream interface hash/version signoff is still partial;
- mainnet deployment has not been broadcast;
- post-deploy verification and smoke evidence do not exist yet;
- Kartal and signer policies have not been switched to an actual deployed V4
  target;
- accepted Kartal V4 dry-run does not exist yet;
- operator/reviewer signoff remains blocked.

## Final Go/No-Go Checklist

Immediately before deploy:

1. Rerun `00_preflight.sh`, `01_build_and_hash.sh`, `02_fork_rehearsal.sh`, and
   `03_generate_calldata.sh` into the final run folder.
2. Recheck deployer nonce and predicted address.
3. Confirm `signoff.json` records operator and reviewer approval.
4. Confirm deploy script still refuses without `CONFIRM_DEPLOY=1`.
5. Broadcast only with the encrypted keystore signer path.

Immediately after deploy:

1. Store deploy output and receipt.
2. Run source verification.
3. Run post-deploy immutable smoke checks.
4. Update Kartal and signer allowlists to the deployed V4 vault address and
   selectors `0x1c2ecb7f` and `0x3a832d08`.
5. Rerun Kartal with `EXPECT=accepted` and store the report.
6. Keep live trading disabled until receipt reconciliation and live planner
   wiring are signed off.
