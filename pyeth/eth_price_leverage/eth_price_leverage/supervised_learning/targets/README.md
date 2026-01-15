# Targets

Definitions and computation logic for supervised prediction targets. Each target module should describe:
- precise mathematical definition (e.g., next-block return, short-horizon realized volatility, liquidation probability)
- labelling window and alignment rules with information timestamps
- any smoothing or filtering applied prior to model consumption
