# Entry Init Policy

This shared rule owns Gate 2 for entry decisions: after a pool passes basic
eligibility, decide whether the pool is fresh enough and clean enough to enter.

The first version is intentionally conservative and data-driven:

- the rule is inert by default;
- age checks only run when `max_age_blocks` or `require_creation_block` is set;
- price/initial checks only run when `max_price_ratio_to_initial` is set;
- missing price/initial data is allowed unless the policy explicitly forbids it.

Thresholds must come from Risk Atlas evidence. Strategy families should not add
hidden price/initial caps outside this rule.
