import os
import pandas as pd
from typing import Set
from functools import lru_cache


ETH_DATA_DIR = os.environ["ETH_DATA_DIR"]
scammers_df_dir = os.path.join(os.environ["ETH_DATA_DIR"], "scammers.parquet") 

scammers_address_set = set(pd.read_parquet(scammers_df_dir)["address"].tolist())


@lru_cache(maxsize=1)
def load_grey_addresses() -> Set[str]:
    return scammers_address_set

def get_grey_addresses() -> Set[str]:
    return load_grey_addresses()


bribe_threshold = 0.1
HIDDEN_MINT_MODEL_PATH = "/home/nima/code/crypto/Aladdin3_Models/scripts/models/bytecode/best_hidden_mint_model.pth"
