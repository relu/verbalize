#!/usr/bin/env python3
"""Differential test of the verbalize RBNF core against ICU4C (design §11.2).

For every vendored locale and every public spellout ruleset in that locale's
CLDR RBNF data, spells the same set of integers through PyICU's
RuleBasedNumberFormat and through the `rbnf_dump` Rust example, and reports
any disagreement. Exits non-zero if any mismatch is found.

Usage: python3 icu_diff.py [--locales de,en,ro] [--stride 997] [--seed 42]
"""
from __future__ import annotations

import argparse
import random
import re
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import icu
except ImportError:
    print(
        "PyICU is not installed; see tools/icu-diff/README.md for setup",
        file=sys.stderr,
    )
    raise SystemExit(2)

DEFAULT_LOCALES = ["de", "en", "ro"]
SOFT_HYPHEN = "\u00ad"
WHITESPACE_RE = re.compile(r"\s+")
NONE_SENTINEL = "<NONE>"


def normalize(s: str) -> str:
    """Soft hyphens removed, whitespace collapsed — same rule the renderer applies (§8)."""
    return WHITESPACE_RE.sub(" ", s.replace(SOFT_HYPHEN, "")).strip()


def public_rulesets(locale: str) -> list[str]:
    rbnf = icu.RuleBasedNumberFormat(icu.URBNFRuleSetTag.SPELLOUT, icu.Locale(locale))
    return [rbnf.getRuleSetName(i) for i in range(rbnf.getNumberOfRuleSetNames())]


def sample_values(stride: int, seed: int) -> list[int]:
    """Full 0-10,000; a stride over 0-1,000,000; powers of ten to 10^18; 10,000 seeded randoms."""
    values = set(range(0, 10_001))
    values.update(range(0, 1_000_001, stride))
    values.update(10**k for k in range(19))  # 10^0 .. 10^18
    rng = random.Random(seed)
    for _ in range(10_000):
        n = rng.randint(0, 10**18)
        if rng.random() < 0.2:
            n = -n
        values.add(n)
    return sorted(values)


def icu_spell(rbnf_cache: dict[str, "icu.RuleBasedNumberFormat"], locale: str, ruleset: str, n: int) -> str | None:
    """Formattable+setInt64 preserves full int64 precision; format(python int) does not
    (PyICU's int overload rounds through a C double, silently losing digits above 2^53)."""
    rbnf = rbnf_cache.setdefault(locale, icu.RuleBasedNumberFormat(icu.URBNFRuleSetTag.SPELLOUT, icu.Locale(locale)))
    f = icu.Formattable()
    f.setInt64(n)
    try:
        rbnf.setDefaultRuleSet(ruleset)
        return rbnf.format(f)
    except icu.ICUError:
        return None  # ICU itself has no answer for this (locale, ruleset, n); not a bug to report


def run_rust(cargo_args: list[str], requests: list[tuple[str, str, int]]) -> list[str | None]:
    with tempfile.TemporaryDirectory() as td:
        infile = Path(td) / "in.txt"
        outfile = Path(td) / "out.txt"
        infile.write_text(
            "\n".join(f"{loc}\t{rs}\t{n}" for loc, rs, n in requests) + "\n", encoding="utf-8"
        )
        with infile.open("r", encoding="utf-8") as fin, outfile.open("w", encoding="utf-8") as fout:
            subprocess.run(["cargo", *cargo_args], stdin=fin, stdout=fout, check=True)
        lines = outfile.read_text(encoding="utf-8").splitlines()
    if len(lines) != len(requests):
        print(
            f"rbnf_dump produced {len(lines)} lines for {len(requests)} requests", file=sys.stderr
        )
        raise SystemExit(2)
    return [None if line == NONE_SENTINEL else line for line in lines]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--locales", default=",".join(DEFAULT_LOCALES))
    parser.add_argument("--stride", type=int, default=997, help="stride over 0..1,000,000 (default keeps runtime sane)")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument(
        "--cargo-args",
        default="run --release -q -p verbalize --example rbnf_dump",
        help="cargo invocation that runs the rbnf_dump example",
    )
    args = parser.parse_args()

    locales = [loc.strip() for loc in args.locales.split(",") if loc.strip()]
    values = sample_values(args.stride, args.seed)

    requests: list[tuple[str, str, int]] = [
        (locale, ruleset, n)
        for locale in locales
        for ruleset in public_rulesets(locale)
        for n in values
    ]

    print(f"testing {len(requests)} (locale, ruleset, n) tuples", file=sys.stderr)
    ours = run_rust(args.cargo_args.split(), requests)

    rbnf_cache: dict[str, "icu.RuleBasedNumberFormat"] = {}
    mismatches = 0
    for (locale, ruleset, n), our in zip(requests, ours):
        expected = icu_spell(rbnf_cache, locale, ruleset, n)
        if expected is None:
            continue  # no ICU ground truth for this tuple
        if our is None or normalize(expected) != normalize(our):
            print(f"MISMATCH locale={locale} ruleset={ruleset} n={n} icu={expected!r} ours={our!r}")
            mismatches += 1

    print(f"{mismatches} mismatch(es) out of {len(requests)} tuples", file=sys.stderr)
    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
