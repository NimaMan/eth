# backdoor_horizon_oot_v1

Out-of-time short-horizon Risk Atlas experiment for scam probability with
explicit token-control and pair-balance backdoor features.

## Ranges

| Split | Blocks | Purpose |
| --- | ---: | --- |
| Train | `24974082..25024081` | Build model rows and fit models. |
| Gap | `25024082..25124081` | Reduce pool/network leakage between train and test. |
| Test | `25124082..25144081` | Out-of-time evaluation; includes ULIQ. |

Any `(token_address, pool_address)` that appears in both train and test is
dropped from the test set before model evaluation.

## Holdout

- Token: `0x0ca72b24abf950be8bb145f5c68f7004485192b2`
- Pool: `0x57ca70b663ead85122589f345835d8f8209608dc`
- Scam block: `25144063`
- Known pre-scam signal: control `transferFrom(holder, dead, amount)` at block
  `25144057`.

## Iteration Loop

Each iteration should update `iterations/iteration_XX.md` with:

1. Train/test rows and overlap removals.
2. Best model per horizon.
3. True positives: what features caught them.
4. False positives: whether they look like missed labels or noisy signals.
5. False negatives: which signal family is missing.
6. ULIQ rank/probability and first flagged block.
7. Feature changes for the next iteration.

## Run Notes

- The 50K train export must run with the release binary:
  `RUST_LOG=error target/release/risk_atlas_headless ... --progress-every-secs 60`.
- Large imports require compact Risk Atlas observation JSON to avoid retaining
  full per-observation feature payloads during import assembly.
- Train import: `3818` pools, `6455` event evidence rows, `1138128`
  observations.
- Test import: `1295` pools, `2013` event evidence rows, `79463`
  observations.
- OOT dataset: `155522` rows, with `101403` train rows, `28130` validation
  rows, `25989` test rows, and `0` overlapping test pools dropped.
- Iteration 01 is documented in `iterations/iteration_01.md`; iterations 02
  through 05 have planned review templates. The notes are published on the Risk
  Atlas modeling page as `backdoor_horizon_oot_v1`.
