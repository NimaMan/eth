# References

Curated artefacts that inform the Baygus Router implementation. Use this folder to store:

- Verified router Solidity sources downloaded from explorers (flattened and/or reconstructed).
- ABI snapshots for routers, PoolManager interfaces, and hook contracts.
- Design notes summarising observed patterns (e.g. settlement flows, fee accounting, hook calls).

## Starting Points

- Uniswap v4 router example:
  [`0x89110a5a3e01760f88966feefa3f5966c2d1c940`](https://etherscan.io/address/0x89110a5a3e01760f88966feefa3f5966c2d1c940#code)
  – inspect how it acquires the PoolManager lock, sequences hook callbacks, and settles payouts.

When adding new files, include a short front-matter block at the top describing where the code came
from, when it was captured, and any modifications you made (e.g. reformatting, removing deployment
metadata). This helps keep provenance clear as we iterate on the Baygus Router.
