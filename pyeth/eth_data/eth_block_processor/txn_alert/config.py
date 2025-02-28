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
    return set(scammers_address_set)

def get_grey_addresses() -> Set[str]:
    return load_grey_addresses()


@lru_cache(maxsize=1)
def load_orca_addresses() -> Set[str]:
    return set(orca_addresses)

def get_orca_addresses() -> Set[str]:
    return load_orca_addresses()


@lru_cache(maxsize=1)
def load_whale_addresses() -> Set[str]:
    return set(whale_addresses)

def get_whale_addresses() -> Set[str]:
    return load_whale_addresses()


whale_addresses = {
        "0x0bb454c2d4e642a5c18f1b3db4d020ba202907df",
        "0x05c6e72a7b7a21e9858749fcd051c9d99c2fb8ea",
        "0x7ede4307271be610b11ce467affa615303390b15",
        "0x3229378620b8318f3012fb32d63363e5d5b22947",
        "0x4582fadad13031ee0f7b3ac70508e54048c7d237",
        "0x1c1769a6a68abf1573840f5925eea59d1875d468",
        "0x4e8c7f243fab07d2ff3761d1ca43414ac6f61d12",
        "0x61a923220e1cf51eae57c7b4b3391c86cb8ad8e3",
        "0xaf30c9f83336b065fcd5c4e4ea19104e6bc54e20",
        "0x6b1e5963022742e2ba9bab0e7432198093afc67d",
        "0xdec64ff1d5033e47a8dacf19144a9bfe55cd3b9e",
        "0x8567f1368d413078aefc31a25484ab59d506e759",
        "0x24eca210afee728eb7e8e35642ed1582ff605a84",
        "0xe310e87110c1bc9444b2689d73cc1146c0a8ef19",
        "0xd059738d3905d78a4aea14f6ed124f9a1ac4a349",
        "0x6bcfa1d946a14fcec05819d98ae2888b9d7d4bc0",
        "0x14a50da15db91c65d1571ad33f81b3a7647d2177",
        "0xde277ba9ee90e11390c421b5dbcf5166b52e4395",
        "0xee80174941b3900507d910b386c7243d20b93abc",
        "0x5b928c7131220e6ce7b10752e752fda9b1e51982",
        "0x694ac550d1f524315a6c430358fc6d4eced327c5",
        "0x2b666e8d289f3ea93a5cf2ad3dd9645c9eb1be58",
        "0x77777639f22bb93b6dca1345ff659f6f8ccd6958",
        "0x1c95c6561d88c996a5b63b181b3f69ad1b7349bb",
        "0x0ed77d94d94f2e41deafc6c7ca5da5f898414f55",
        "0x555565d92153226d3bdf6adb1a9b5cab77a16dca",
        "0x323b6ffea4efec08e50bc7a80c5c02ad1d8b87b9",
        "0xc570fa7b5b14ba34315ba99d29e41defd3cb1f2b",
        "0x40a407e868f0b77a56ec829b3b30f92f8a50646c",
        "0xdec1a706d52a02ff0b51d951204d2d98230685c9",
        "0x35dff34ccb256c8154284a813f26f99d8821aab3",
        "0x24d2a60d9d77e5aaa898f47eaad0730f74c02095",
        "0xc403fcee04bc99e5867512a1d35c78be7a6d82f0",
        "0x01f124de00c0103bd34b8f1d8640c3c6ee44c016",
        "0x418feca30be193b95da08e3b3a227612e3149350",
        "0x025d13089ca77e64ad767085f8dfdc1162a75e80",
        "0x192a25d7aa74a1bf15192ae6e3170276eced6f31",
        "0x25e329aa5264fca7139715eaf735fc369c49d568",
        "0x5c1d7b605398d5418a4d9fa901acff6f1cdf7f47",
        "0xcde567e3a91235688829c83f5e75f45e94d4fa41",
}


orca_addresses = {
    "0xa53476642994Ada66E59E7cc9F7754A3862A44c2",
    "0x00D78DAF782921B27a6b407d34F19842C10a4a6B",
    "0x3C7E7e5D95a154764E209e6BCac1F0653fEE75AC",
    "0x8A494eD488C5BC157E0Fb2983FdFb90E2d63B691",
    "0xd168343dA56d890656045B8530982bc13EBfD38b",
    "0xc1950C51D59A896DA027fb4698F58E6D5CE7cbc7",
    "0xBdf59d84e393cC0728409144512D2cd62C3278f3",
    "0x0144b7Bc5344F79905385bE705d7dE371Ab81463",
    "0x8bf69e5e12cF2a0feFD20e5fC6377e4714564808",
    "0xc33c9692A00608502F84AEEcA2cfeeA887c84Ee8",
    "0x49A840daf2a29cd327065A878D75dE95E4007C6A",
    "0x6224fE0c8067ddB65C8c45eeD7C0636511a0f98B",
    "0xd8600EF4B0A2b4ed5f4aeaab7CE023A811F97406",
    "0xC64af4852814dC74d43BE97d4Ab2870844e343dA",
    "0x9Ed5e9763FAF84cFe4bA2FF9cBb73e8f1f6c6314",
    "0xbae88c80d8B362Bbd09a39b60dd5eb15b3eD36B0",
    "0xC0bA61F08E16E6b86494c810a2610356d5f3Ba56",
    "0xCf78282aFf3180927F1F2f153582aAd84E651d3F",
    "0xDAc63C762feE117191DC1e8E6E52e5F5D0d1361f",
    "0x1071316b698da9947b2F267A02fD77e17ef2Fb5b",
    "0xb51074B09f8799e597E9F39178cAeD303F26433A",
    "0xa2C1E1129a63498ce6CD81ce2c31d50eA29E16a1",
    "0xb0521f39826aD3ceAE124c02cBed997cffc31d87",
    "0x4bfd7285b3341DFA461bD9914bBC67Ce1832c532",
    "0x193e75b60A4Ca8BC842Dc28604Afc6c41aFE972A",
    "0x1f25305979adb218F795C27757D5F86aB9A81179",
    "0xe5cfE71F783d3Bff5736F4Be1BC44cf58B812E62",
    "0xa86e9254D3f3e3c6a6d89B1Dbe6Bb61eE53d2245",
    "0x8dcB8be7BD4728Dbc774D743765B5F2A0385F88a",
    "0x4B412726C511dB52bd22E3e6415294D12cC2b0Bb",
    "0x6749c8E2Cf3FECA809314338E68489fdCd250fF8",
    "0xE17eAA1E3295eB9105be0BFc3d96D545b6e92534",
    "0xFB169e6Cf0368Bb7C94A055a38341a29151905b3",
    "0xBc21f120e86f55D4981154f41B16d9927e16b9Ef",
    "0x82405fFB18c7746F802cB0a63891CBf02842CA1C",
    "0x9E1A2B718A24f6005b643C964465e02cB3C17138",
    "0x553e6079e46eFE88827CFB0287B695339662341c",
    "0x88c26DDa0d6A0BB2CC87780b5DfE7FAA574018aF",
    "0xBAD78cB2Ac1E4b7Ca62d751404E13B9B6CaE284B",
    "0x57783B4397f3415EA87328f1688818637f0A49E4",
    "0x91E521aB0B2A83E56E4A9Ef86748Aa38e5C1dfdb",
    "0xE975934D168862040AAbC70C86C619F0045020CA",
    "0xdF435Aa871532A237772b619A03e2ca8b9162193",
    "0x5dD21d9e20949dBcDCc9cA3123aC7335f3f9296F",   
}