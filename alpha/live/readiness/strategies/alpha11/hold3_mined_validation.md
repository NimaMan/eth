# Alpha11 Hold3 Mined Validation

This is the one-position mined-validation instance for Alpha11.

## Strategy

`alpha11-live-univ2-lp30-pool-update-block-hold3-validation`

This validation strategy keeps the main Alpha11 entry surface but shortens the
hold window to 3 pool-update blocks and limits the run to one entry. It exists
to prove one mined buy, one mined sell, receipt reconciliation, tx index, actual
gas cost, deployed-vault event handling, and confirmation recheck before
enabling the main hold15 strategy.

## Expected Spec-Owned Parameters

- Protocol scope: Uniswap V2.
- LP approval gate: 30%.
- Entry trigger: pool-update-block path from the Alpha11 live strategy.
- Buy size: `0.01 ETH`.
- Entry bankroll: `0.01 ETH`.
- Max entry pools: `1`.
- Hold window: 3 pool-update blocks.
- Deployed V2 vault: `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`.
- Runtime strategy parameters must not be supplied by CLI flags.

## Current Evidence

The latest public-validation attempt reached Kartal and the signer boundary but
did not pass mined validation.

| Field | Value |
| --- | --- |
| Run id | `alpha11-hold3-public-validation-20260521-171525Z` |
| Trade id | `trd_mpfr53qi_137y0_1` |
| Token | `0x9a95361A959e01d7F55Ab43D1D76a7190eDF8D4a` |
| Result | `buy_failed` |
| Useful proof | Exact deployed V2 vault calldata simulation passed and the request reached signer policy. |
| Failure reason | Signer value cap was `0`, and worst-case transaction cost exceeded the signer cap. |

The rejected buy sent `0.01 ETH` to the deployed vault. The prepared gas envelope
was `300000` gas at `50 gwei` max fee, so the worst-case cost was:

```text
0.01 ETH + 300000 * 50 gwei = 0.025 ETH
```

That attempt is a valid failed-gate artifact. It does not count as a passed
mined-validation trade because no transaction was signed, broadcast, mined, or
reconciled.

## Remaining To Pass

- Align Kartal executor caps with the validation tx shape.
- Align signer caps with the validation tx shape.
- Run final dry-run signing check.
- Switch only Kartal broadcast mode to `public_mempool`.
- Run this strategy by name with the public-mempool validation guard.
- Collect buy and sell receipt evidence.
- Review Asena, alpha DB, Kartal journal, signer journal, RPC receipts, and
  deployed-vault events.

