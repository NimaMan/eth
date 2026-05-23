# Artifacts

Generated artifacts for this investigation should live here and should not be
committed unless they are small, stable, and needed for reproduction.

Useful reproduction commands:

```bash
cast receipt --rpc-url http://127.0.0.1:8545 \
  0x6c07721ee377840bf4c9720c4693b704646ba8ac89efab9c090329e1664a41e3

cast receipt --rpc-url http://127.0.0.1:8545 \
  0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697

cast logs --rpc-url http://127.0.0.1:8545 \
  --from-block 25077324 --to-block 25081722 \
  --address 0xd8e654a9b4e861b54adb903ada20e3787dbd0079 \
  'Transfer(address,address,uint256)'

cast logs --rpc-url http://127.0.0.1:8545 \
  --from-block 25077595 --to-block 25077595 \
  --address 0xd8e654a9b4e861b54adb903ada20e3787dbd0079 \
  'Burn(address,uint256,uint256,address)'
```
