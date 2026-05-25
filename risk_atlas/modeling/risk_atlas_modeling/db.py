"""Database access helpers for Risk Atlas modeling extracts."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pandas as pd
import psycopg2


ETH_ROOT = Path(__file__).resolve().parents[4]
TOML_CONFIG_PATH = ETH_ROOT / "config.toml"


def resolve_database_url() -> str:
    value = _config_value("databases.risk_atlas.url")
    if value:
        return value
    raise RuntimeError(f"{TOML_CONFIG_PATH} must define databases.risk_atlas.url")


def connect():
    return psycopg2.connect(resolve_database_url())


def read_sql(sql: str, params: dict) -> pd.DataFrame:
    with connect() as conn:
        return pd.read_sql_query(sql, conn, params=params)


def fetch_run_bounds(run_id: str) -> dict:
    with connect() as conn, conn.cursor() as cur:
        cur.execute(
            """
            SELECT start_block, end_block, block_count
            FROM public.risk_atlas_runs
            WHERE run_id = %s
            """,
            (run_id,),
        )
        row = cur.fetchone()
    if not row:
        raise RuntimeError(f"Risk Atlas run not found: {run_id}")
    return {
        "start_block": int(row[0]),
        "end_block": int(row[1]),
        "block_count": int(row[2]),
    }


def _config_value(key: str) -> str | None:
    if not TOML_CONFIG_PATH.is_file():
        return None
    data = tomllib.loads(TOML_CONFIG_PATH.read_text(encoding="utf-8"))
    value: object = data
    for part in key.split("."):
        if not isinstance(value, dict):
            return None
        value = value.get(part)
    return value.strip() if isinstance(value, str) and value.strip() else None
