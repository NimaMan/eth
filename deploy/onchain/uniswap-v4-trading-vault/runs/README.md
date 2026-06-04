# V4 Vault Runs

Each V4 assessment or deployment attempt gets a dated subfolder here.

The first real run must copy `TEMPLATE/` and fill every evidence file before
mainnet broadcast is allowed.

Current pre-deploy gate run:

- `20260519-v4-gates-202032Z/`: build/hash, fork rehearsal, tx_simulator
  comparison, constructor/init-code calldata, and ETH tx executor policy-rejection
  evidence. It does not contain a mainnet deployment receipt.
