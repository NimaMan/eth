import os
import pandas as pd
from typing import Set
from functools import lru_cache


ETH_DATA_DIR = os.environ["ETH_DATA_DIR"]
scammers_df_dir = os.path.join(os.environ["ETH_DATA_DIR"], "scammers.parquet") 

scammers_address_set = set(pd.read_parquet(scammers_df_dir)["address"].tolist())
HIDDEN_MINT_MODEL_PATH = "/home/nima/code/crypto/Aladdin3_Models/scripts/models/bytecode/best_hidden_mint_model.pth"


bribe_threshold = 0.1


@lru_cache(maxsize=1)
def load_grey_addresses() -> Set[str]:
    return scammers_address_set

def get_grey_addresses() -> Set[str]:
    return load_grey_addresses()


@lru_cache(maxsize=1)
def load_orca_addresses() -> Set[str]:
    return []

def get_orca_addresses() -> Set[str]:
    return load_orca_addresses()


@lru_cache(maxsize=1)
def load_whale_addresses() -> Set[str]:
    return []

def get_whale_addresses() -> Set[str]:
    return load_whale_addresses()


