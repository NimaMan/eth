from setuptools import setup, find_packages

NAME="eth_token"
VERSION="0.1.0"
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
    description="eth_token",
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
    eth_token is a package for analyzing and monitoring Ethereum tokens.
    """
)



'''
v0.2.0
 - 


v 0.1.0 - Initial release
Initial release of the eth_token packag with the following modules:
- live_erc20_token
- token_health
- token_network

Version History
'''