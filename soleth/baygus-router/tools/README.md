# Tools

Utility scripts that support the Baygus Router roadmap live here. Keep each script self-contained
and describe its purpose, inputs, and expected output.

## Available Scripts

- `replay_moonstr_swap.py` – decodes transaction `0x6910…c9` and prints the asset movements that
  occurred through the Uniswap v4 PoolManager. This is the acceptance artifact for v0.1 of the
  roadmap.

## Usage

```
python sol/baygus-router/tools/replay_moonstr_swap.py
```

Scripts should not require network access; they rely on recorded data stored in `references/`.
