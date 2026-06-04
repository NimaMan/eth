# Checklist

Use this checklist for every one-position mined-validation run.

## Preflight

- The selected strategy has a short-hold, one-entry validation spec.
- Strategy economics are selected by strategy name, not CLI overrides.
- ETH tx executor reports healthy status.
- ETH tx executor starts in `broadcast_mode = dry_run`.
- ETH tx executor policy has the expected `allowed_from`, `allowed_targets`,
  and `allowed_selectors`.
- Signer policy has matching `allowed_targets` and `allowed_selectors`.
- The signer address has enough ETH for the validation buy and sell.
- Alpha strategy bankroll has enough available balance for one buy and one sell;
  ETH tx executor daily spend cap is disabled for this launch.
- Exact final calldata simulation targets the deployed vault route.
- Simulation freshness is within the configured block limit.

## Cap Checks

- For a buy that sends ETH, ETH tx executor and signer `max_value_wei` must be
  at least the buy value.
- ETH tx executor and signer `max_transaction_cost_wei` must cover:

```text
tx.value + gas_limit * max_fee_per_gas
```

- ETH tx executor and signer fee caps must allow the selected EIP-1559 gas plan.
- ETH tx executor and signer gas-limit caps must allow the prepared tx gas limit.
- Spend reservations must be released for dry-runs, rejects, and broadcast
  failures that did not submit an on-chain transaction.

Example for a `0.005 ETH` buy, `300000` gas limit, and `50 gwei` max fee:

```text
0.005 ETH + 300000 * 50 gwei = 0.020 ETH worst-case cost
```

## Dry-Run Gate

- Run strategy-specific dry-run calibration and keep the report artifact.
- Run one final dry-run signing check after cap changes.
- A policy or signer rejection is a valid failed gate only if the exact request
  and journal reason are recorded.
- Dry-run signing does not count as mined validation.

## Public Broadcast Gate

- Switch only ETH tx executor broadcast mode to `broadcast`.
- Do not change strategy parameters by CLI flags.
- Run the validation strategy by name with the explicit broadcast guard.
- Stop the validation run after one complete buy/sell lifecycle or after a
  documented rejection/failure.

## Pass Criteria

- ETH tx executor signs and broadcasts one buy transaction.
- Alpha records `buy_submitted` only from the ETH tx executor broadcast response.
- Receipt reconciliation records `buy_confirmed` only from a successful mined
  receipt with the expected deployed-vault event.
- The strategy emits the short-hold sell from the confirmed position.
- ETH tx executor signs and broadcasts the sell transaction.
- Receipt reconciliation records `sell_confirmed` from the mined sell receipt.
- Persisted evidence includes tx hash, block number/hash, transaction index,
  gas used, effective gas price, paid gas, vault event amounts, and the
  configured confirmation recheck.
