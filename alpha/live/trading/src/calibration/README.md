# Kartal Calibration

This module owns repeatable dry-run checks for the alpha -> Kartal execution
boundary.

The calibration runner submits prepared `eth_direct_raw_v1` requests to Kartal,
then records:

- authenticated `/eth/tx/status`
- submit result or HTTP rejection body
- `/eth/tx/policy/decisions/{attempt_id}` journal entries
- a machine-readable verdict

By default the runner refuses to submit unless Kartal reports
`broadcast_mode = dry_run`. This is deliberate: the calibration suite should be
safe to run repeatedly.

## Safe Default Check

The included fixture expects Kartal's reject-by-default policy to reject before
signing. It does not need a signer:

```bash
cd /home/nima/code/crypto/blockchains/eth
ETH_TX_EXECUTOR_API_TOKEN=... cargo run -p eth_alpha_engine --bin eth_alpha_kartal_calibrate -- \
  --request alpha/live/trading/fixtures/kartal_calibration/reject_policy_request.json
```

## Full Dry-Run Signing Check

After configuring a tiny hot-wallet policy and a dry-run signer in Kartal, use a
planner-produced request and expect a signed dry-run:

```bash
cargo run -p eth_alpha_engine --bin eth_alpha_kartal_calibrate -- \
  --planner-fixture \
  --planner-fixture-from 0x... \
  --expect dry-run-signed \
  --refresh-simulation-block
```

That check proves decode, auth, policy acceptance, spend reservation, signing,
tx_executor journaling, and no broadcast.

Planner-fixture requests append a UTC timestamp to the generated `attempt_id`
by default so repeated dry-run signing checks remain distinct in Kartal's
policy journal and spend ledger. Use `--stable-planner-fixture-attempt-id` only
when a deterministic request id is required.

To inspect or policy-allow the exact planner-produced target and calldata
selector first, write the request without contacting Kartal:

```bash
cargo run -p eth_alpha_engine --bin eth_alpha_kartal_calibrate -- \
  --planner-fixture \
  --planner-fixture-from 0x... \
  --write-request-path /tmp/kartal-planner-produced-request.json \
  --write-request-only
```
