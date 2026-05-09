# SSS SpaceX Space Services

This case tracks the SSS pool whose displayed price ratio became extremely high
while the pool was also marked `cannot_sell`.

The first objective is parity, not classification:

1. Extract the chain truth for the pool and key transactions.
2. Replay the same transactions with our simulator.
3. Confirm whether the simulator can reproduce the chain behavior.
4. Decide which odd behavior detectors this case should produce.

## Addresses

- token: `0x144742cc48cacb0f2e49dd03af5cfca3fa30e7bc`
- pool: `0x06f53d7c71f91060d7e698218d78ff293f4f2aa5`
- denom: `WETH`
- creator: `0xd962715b742d7a761a09a5141d85679e8472a6b0`

## Range

- start block: `25,041,048`
- end block: `25,041,137`
