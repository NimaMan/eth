from setuptools import setup, find_packages

NAME="eth_block_processor"
VERSION="0.2.0"
# To install the library, run the following
#
# python setup.py install
#
# prerequisite: setuptools
# http://pypi.python.org/pypi/setuptools

REQUIRES=[
]

setup(
    name=NAME,
    version=VERSION,
    description="eth_block_processor",
    author="Nima",
    author_email="",
    url="",
    keywords=[""],
    python_requires=">=3.10",
    install_requires=REQUIRES,
    packages=find_packages(exclude=["tests"]),
    package_data={'': ['abis/*.json', 'apis/api_keys/*.yaml', 'db/config.yaml']},
    include_package_data=True,
    long_description="""\
    eth_block_processor provides ...
    """
)


'''
Change log 
0.1.0: 
- Initial release with the basic functionality:
    - Process the transactions  
        - Uniswap v2 
    - Process the blocks
    - Publish the processed blocks for the other components
    - Basic alerts
        - contract creation
        - trading enabled

0.2.0: 
- Add uniswap v3 positions

'''