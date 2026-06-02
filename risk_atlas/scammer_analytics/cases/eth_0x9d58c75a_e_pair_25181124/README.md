# 0x9d58c75a e Pair Case

Actor-centric case for the `e` token/pair incident around block `25181124`.

## Scope

- suspect: `0x9d58c75A8749a94C9CF9539B4f625Ed42936105D`
- token: `0x2CedB62299Ad3422Fc0b0ec57180695501D15fef`
- pool: `0x5D43262637B4fc4bfaF4164A8974c436D5CFCe8D`
- forwarder: `0x14b7b488ada20b3b3ec6735d212cf9bb03e5f9c4`

## Current Hypothesis

The suspect wallet is not a disposable nonce-zero creator. It is an active
operator wallet that repeatedly deploys same-name/same-bytecode `e` contracts,
funds them with ETH, opens liquidity, removes liquidity, and distributes ETH
through a CREATE/selfdestruct forwarding contract.

## Run

```bash
cargo run -p eth_risk_atlas -- scammer-case \
  risk_atlas/scammer_analytics/cases/eth_0x9d58c75a_e_pair_25181124/case.toml
```
