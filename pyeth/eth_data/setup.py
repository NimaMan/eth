from setuptools import setup, find_packages

NAME="ethblockprocessor"
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
    description="EthBlockProcessor",
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
    EthBlockProcessor provides ...
    """
)