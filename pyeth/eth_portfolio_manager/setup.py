from setuptools import setup, find_packages

NAME="eth_portfolio_manager"
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
    description="eth_portfolio_manager",
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
    eth_portfolio_manager is our portfolio manager.
    """
)


'''
Change log 
0.1.0: 
- Initial release with the basic functionality:
    - Created Portfolio Manager that:
      - processes the updated tokens from the live token manager
      - puts the updated positions into a redis server
      - Creates a simple buy everything strategy 

0.1.1: 
- Shows the current portfolio state in a simple way


0.1.2: 
- Creates a position manager that:
    - Strategies based on the current alerts in 
        - eth_block_processor
        - eth_token
    - publishes a decision to the txn manager

'''