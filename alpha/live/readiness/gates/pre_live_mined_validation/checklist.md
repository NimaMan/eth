# Checklist

Use this checklist for every one-position mined-validation run.

## Preflight

- The selected strategy has a short-hold, one-entry validation spec.
- Strategy economics are selected by strategy name, not CLI overrides.
- Kartal reports healthy status.
- Kartal starts in `broadcast_mode = dry_run`.
- Kartal executor policy has the expected `allowed_from`, `allowed_targets`,
  and `allowed_selectors`.
- Signer policy has matching `allowed_targets` and `allowed_selectors`.
- The signer address has enough ETH for the validation buy and sell.
- Daily spend remaining is enough for one buy and one sell.
- Exact final calldata simulation targets the deployed vault route.
- Simulation freshness is within the configured block limit.

## Cap Checks

- For a buy that sends ETH, Kartal and signer `max_value_wei` must be at least
  the buy value.
- Kartal and signer `max_transaction_cost_wei` must cover:

```text
tx.value + gas_limit * max_fee_per_gas
```

- Kartal and signer fee caps must allow the selected EIP-1559 gas plan.
- Kartal and signer gas-limit caps must allow the prepared tx gas limit.
- Spend reservations must be released for dry-runs, rejects, and broadcast
  failures that did not submit an on-chain transaction.

Example for a `0.01 ETH` buy, `300000` gas limit, and `50 gwei` max fee:

```text
0.01 ETH + 300000 * 50 gwei = 0.025 ETH worst-case cost
```

## Dry-Run Gate

- Run strategy-specific dry-run calibration and keep the report artifact.
- Run one final dry-run signing check after cap changes.
- A policy or signer rejection is a valid failed gate only if the exact request
  and journal reason are recorded.
- Dry-run signing does not count as mined validation.

## Public Mempool Gate

- Switch only Kartal broadcast mode to `public_mempool`.
- Do not change strategy parameters by CLI flags.
- Run the validation strategy by name with the explicit public-mempool guard.
- Stop the validation run after one complete buy/sell lifecycle or after a
  documented rejection/failure.

## Pass Criteria

- Kartal signs and broadcasts one buy transaction.
- Alpha records `buy_submitted` only from the Kartal broadcast response.
- Receipt reconciliation records `buy_confirmed` only from a successful mined
  receipt with the expected deployed-vault event.
- The strategy emits the short-hold sell from the confirmed position.
- Kartal signs and broadcasts the sell transaction.
- Receipt reconciliation records `sell_confirmed` from the mined sell receipt.
- Persisted evidence includes tx hash, block number/hash, transaction index,
  gas used, effective gas price, paid gas, vault event amounts, and the
  configured confirmation recheck.

