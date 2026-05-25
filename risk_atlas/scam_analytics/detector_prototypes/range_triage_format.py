from __future__ import annotations

import json
import textwrap
from collections import Counter
from collections.abc import Iterable, Sequence
from typing import Any

from range_triage_utils import SEVERITY_RANK, clean_json, short_hash


def format_json(snapshot: RangeRunSnapshot, candidates: list[IssueCandidate]) -> str:
    payload = {
        "run_id": snapshot.run_id,
        "api_base": snapshot.api_base,
        "progress": snapshot.progress,
        "counts": counts(candidates),
        "candidate_count": len(candidates),
        "candidates": [candidate.as_json() for candidate in candidates],
    }
    return json.dumps(clean_json(payload), indent=2, sort_keys=True)


def format_jsonl(candidates: list[IssueCandidate]) -> str:
    return "\n".join(
        json.dumps(clean_json(candidate.as_json()), sort_keys=True) for candidate in candidates
    )


def format_markdown(snapshot: RangeRunSnapshot, candidates: list[IssueCandidate]) -> str:
    severity_counts = Counter(candidate.severity for candidate in candidates)
    kind_counts = Counter(candidate.kind for candidate in candidates)
    lines = [
        f"# Range Triage: `{snapshot.run_id}`",
        "",
        f"- API: `{snapshot.api_base}`",
        f"- Range: `{snapshot.start_block}` through `{snapshot.end_block}`",
        f"- Status: `{snapshot.progress.get('status')}`",
        f"- Tokens: `{len(snapshot.tokens)}`",
        f"- Pools: `{len(snapshot.pools)}`",
        f"- Candidates: `{len(candidates)}`",
        "",
        "## Severity Counts",
        "",
    ]
    if severity_counts:
        for severity, count in sorted(
            severity_counts.items(), key=lambda item: -SEVERITY_RANK.get(item[0], 0)
        ):
            lines.append(f"- `{severity}`: {count}")
    else:
        lines.append("- none")

    lines.extend(["", "## Top Issue Kinds", ""])
    for kind, count in kind_counts.most_common(20):
        lines.append(f"- `{kind}`: {count}")
    if not kind_counts:
        lines.append("- none")

    lines.extend(
        [
            "",
            "## Candidates",
            "",
            "| Severity | Kind | Token | Pool | Protocol | Evidence | Next Step |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for candidate in candidates:
        lines.append(
            "| "
            + " | ".join(
                [
                    md(candidate.severity),
                    md(candidate.kind),
                    md(display_token(candidate)),
                    md(short_hash(candidate.pool_address)),
                    md(candidate.protocol or "-"),
                    md("; ".join(candidate.evidence[:5])),
                    md(candidate.suggested_next_step),
                ]
            )
            + " |"
        )
    return "\n".join(lines)


def format_table(candidates: list[IssueCandidate]) -> str:
    rows = [
        (
            candidate.severity,
            candidate.kind,
            display_token(candidate),
            short_hash(candidate.pool_address),
            "; ".join(candidate.evidence[:3]),
        )
        for candidate in candidates
    ]
    widths = (9, 46, 20, 13, 72)
    header = ("severity", "kind", "token", "pool", "evidence")
    lines = [format_row(header, widths), format_row(tuple("-" * width for width in widths), widths)]
    lines.extend(format_row(row, widths) for row in rows)
    return "\n".join(lines)


def format_row(values: Sequence[str], widths: Sequence[int]) -> str:
    return "  ".join(str(value)[:width].ljust(width) for value, width in zip(values, widths))


def display_token(candidate: IssueCandidate) -> str:
    symbol = candidate.symbol or "-"
    address = short_hash(candidate.token_address)
    return f"{symbol} {address}".strip()


def md(value: Any) -> str:
    text = str(value if value is not None else "-")
    text = text.replace("|", "\\|")
    return textwrap.shorten(text, width=220, placeholder="...")


def counts(candidates: Iterable[IssueCandidate]) -> dict[str, Any]:
    candidates = list(candidates)
    return {
        "by_severity": dict(Counter(candidate.severity for candidate in candidates)),
        "by_kind": dict(Counter(candidate.kind for candidate in candidates)),
    }
