# Live Strategy Sets

A strategy set is a named bundle of one or more live strategy specs.

The live trader accepts a set name from CLI/config, the registry resolves that
name, and the returned specs are persisted as concrete strategy rows for the
run. Default exit behavior belongs to the shared strategy config, not to an
individual set.
