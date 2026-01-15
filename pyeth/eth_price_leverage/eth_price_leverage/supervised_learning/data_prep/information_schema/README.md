# Information Schema

Central registry describing each information element consumed by supervised models. Entries should specify:
- canonical name and version
- dimensionality and units
- refresh cadence and lag assumptions
- upstream node-native sources and dependencies
- validation rules (ranges, nullability, integrity checks)

Schema definitions keep extractors and datasets synchronized as new information types are added.
