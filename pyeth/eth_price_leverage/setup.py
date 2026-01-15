from setuptools import setup, find_packages
from pathlib import Path

ROOT = Path(__file__).parent
readme = (ROOT / "README.md").read_text(encoding="utf-8") if (ROOT / "README.md").exists() else ""

setup(
    name="eth_price_leverage",
    version="0.1.0",
    description="Python agents, policies, and env wrappers for Rust-backed stablecoin RL via pyreth",
    long_description=readme,
    long_description_content_type="text/markdown",
    author="Baygus Team",
    packages=find_packages(),
    python_requires=">=3.9",
    install_requires=[],  # pyreth is a local binary module; ensure it's installed separately
    include_package_data=True,
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
)

