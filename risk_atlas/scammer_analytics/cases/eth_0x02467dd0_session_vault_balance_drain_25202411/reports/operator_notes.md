# Operator Notes

## Drain Mechanism

`cast run --rpc-url http://127.0.0.1:8545 0xf0e8542a57c488121f18bdad4f148799e59ee21b83caa12bdc5b9aadac43d8ba --quick`
decoded the scam transaction as a call from the token owner to
`0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4::multicall(...)`.

The call arguments were:

```text
token = 0xc3A640bD249381F8097f44C1b61C46172068cDff
holders = [0x28474cbCd780AeEb3ED1501B68254bEd87cF5597]
amount = 93
decimals = 9
```

The trace showed:

```text
balanceOf(0x28474cbCd780AeEb3ED1501B68254bEd87cF5597)
  -> 9871580343970612 raw

transferFrom(
  0x28474cbCd780AeEb3ED1501B68254bEd87cF5597,
  0x000000000000000000000000000000000000dEaD,
  9871487343970612 raw
)
  -> true
```

The token emitted `Approval(vault, control_contract, 0)` but no ordinary
`Transfer` event for the burned amount.

## Balance Evidence

Vault allowance to the control contract was zero before and after the drain:

```text
block 25202409: allowance=0 balance=9871580343970612 raw
block 25202410: allowance=0 balance=9871580343970612 raw
block 25202411: allowance=0 balance=93000000000 raw
block 25202412: allowance=0 balance=93000000000 raw
```

With token decimals `9`, the vault balance changed from
`9,871,580.343970612` tokens to `93` tokens. The manual exit sold only `93`
tokens because that was the actual on-chain `balanceOf(vault)` by then.

## System Gaps

- Real receipt reconciliation stored the buy token amount with `18` decimals in
  the live trade payload, even though the token has `9` decimals.
- The live backtest kept valuing the original simulated inventory from pool
  reserves after the drain.
- Risk events captured the LP approval warning at block `25202408`, but did
  not emit a post-entry holder-balance drain signal at block `25202411`.
