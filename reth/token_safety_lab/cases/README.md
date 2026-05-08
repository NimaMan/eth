# Cases

Each case is a concrete token or pool investigation. Cases should be small
enough to reproduce and specific enough to become regression tests.

Use a stable folder name:

```text
<token_symbol_or_name>_<short_context>_<start_block>
```

Examples:

```text
sss_space_services_25041048
biba_primary_v2_pool_25039257
```

Cases are not just notes. A complete case should explain:

- what the range builder reported;
- what the chain actually did;
- whether the simulator can reproduce it;
- whether a detector or guardrail should exist before trading;
- what code changed because of the investigation.

## Candidate Ledger

This is the working triage ledger for behavior that does not make sense yet.
Rows here are not conclusions. They are prompts for chain-truth and simulator
parity investigation.

| Status | Token | Pool | Range / Block | Symptom | Why It Matters | Case |
| --- | --- | --- | --- | --- | --- | --- |
| new |  |  |  |  |  |  |

Status values:

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `ignored`: not useful after review.
