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
