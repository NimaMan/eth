from pathlib import Path
import yaml
from typing import Tuple
from os import listdir

path = Path('api_keys')

def get_api_key(name: str, index: int=0) -> Tuple[str, int]:
    '''Returns the API key at the given index from the yaml file with the given name.
    :param name: The name of the yaml file. The file must be in the 'api_keys' folder.
    :param index: The index of the API key to return.
    :return: The API key at the given index from the yaml file with the given name and the next index.
    '''
    with open(path / f'{name}.yaml', 'r') as file:
        keys = list(yaml.safe_load_all(file))
        return keys[index]['Key'], index + 1 if index < len(keys) - 1 else 0

api_keys = {f[:-5]: get_api_key(f[:-5])[0] for f in listdir(path) if f.endswith('.yaml')}
next_indices = {f[:-5]: get_api_key(f[:-5], 0)[1] for f in listdir(path) if f.endswith('.yaml')}

def alternate_api_keys(name='') -> None:
    global next_indices
    for api_key in api_keys:
        api_keys[api_key], next_indices[api_key] = get_api_key(api_key, next_indices[api_key])