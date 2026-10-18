#!/usr/bin/env python3
"""Differential test of the verbalize RBNF core against ICU4C.

For every vendored locale and every public spellout ruleset ICU lists for it,
first checks that ICU and the vendored data name the same public rulesets (a
ruleset on only one side is a failure), then spells the same set of integers through PyICU's
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
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

try:
    import icu
except ImportError:
    print(
        "PyICU is not installed; see tools/icu-diff/README.md for setup",
        file=sys.stderr,
    )
    raise SystemExit(2)

DEFAULT_LOCALES = ["de", "en", "ro"]
DATA_DIR = Path(__file__).resolve().parents[2] / "verbalize" / "src" / "data"
# A generated table entry: `Ruleset { name: "%spellout-numbering", rules: &[`.
RULESET_NAME_RE = re.compile(r'Ruleset \{ name: "([^"]+)"')
SOFT_HYPHEN = "\u00ad"
WHITESPACE_RE = re.compile(r"\s+")
NONE_SENTINEL = "<NONE>"


def normalize(s: str) -> str:
    """Soft hyphens removed, whitespace collapsed — same rule the renderer applies."""
    return WHITESPACE_RE.sub(" ", s.replace(SOFT_HYPHEN, "")).strip()


def public_rulesets(locale: str) -> list[str]:
    rbnf = icu.RuleBasedNumberFormat(icu.URBNFRuleSetTag.SPELLOUT, icu.Locale(locale))
    return [rbnf.getRuleSetName(i) for i in range(rbnf.getNumberOfRuleSetNames())]


def vendored_public_rulesets(locale: str) -> list[str]:
    """Public (single `%`) ruleset names in the shipped table for `locale`.

    Read from the generated source because the crate exposes no listing; the
    table is checked in sync with the vendored CLDR data by `cargo xtask check-data`."""
    path = DATA_DIR / f"{locale}.rs"
    if not path.is_file():
        print(f"no vendored data for locale {locale!r} ({path})", file=sys.stderr)
        raise SystemExit(2)
    names = RULESET_NAME_RE.findall(path.read_text(encoding="utf-8"))
    if not names:
        print(f"found no rulesets in {path}; has the generated format changed?", file=sys.stderr)
        raise SystemExit(2)
    return [name for name in names if not name.startswith("%%")]


def ruleset_drift(icu_names: list[str], vendored_names: list[str]) -> tuple[list[str], list[str]]:
    """(only in ICU, only in the vendored data): names the comparison would never exercise
    or never request, since requests are built from ICU's list."""
    return sorted(set(icu_names) - set(vendored_names)), sorted(set(vendored_names) - set(icu_names))


def sample_values(stride: int, seed: int) -> list[int]:
    """0-10,000 and their negatives (the `-x` rules); a stride over 0-1,000,000; ±10^k to 10^18;
    10,000 seeded randoms, a fifth of them negative."""
    powers = {10**k for k in range(19)}
    values = set(range(0, 10_001))
    values.update(range(0, 1_000_001, stride))
    values.update(powers)
    values.update(-n for n in [*range(0, 10_001), *powers])
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


@dataclass
class Tally:
    """Outcome counts for one (locale, ruleset)."""

    requested: int = 0
    compared: int = 0  # ICU produced an answer, so there is a ground truth to compare against
    icu_errors: int = 0  # ICU refused (unknown ruleset, or no rule for this value); nothing compared
    mismatches: int = 0


def compare(
    requests: list[tuple[str, str, int]],
    ours: list[str | None],
    icu_answer: Callable[[str, str, int], str | None],
) -> tuple[dict[tuple[str, str], Tally], list[str]]:
    """Compares each of our answers with ICU's, tallying outcomes per (locale, ruleset)."""
    tallies: dict[tuple[str, str], Tally] = {}
    mismatch_lines: list[str] = []
    for (locale, ruleset, n), our in zip(requests, ours):
        tally = tallies.setdefault((locale, ruleset), Tally())
        tally.requested += 1
        expected = icu_answer(locale, ruleset, n)
        if expected is None:
            tally.icu_errors += 1
            continue
        tally.compared += 1
        if our is None or normalize(expected) != normalize(our):
            tally.mismatches += 1
            mismatch_lines.append(f"MISMATCH locale={locale} ruleset={ruleset} n={n} icu={expected!r} ours={our!r}")
    return tallies, mismatch_lines


def vacuous(tallies: dict[tuple[str, str], Tally]) -> list[tuple[str, str]]:
    """Rulesets for which no comparison was made; a renamed ruleset must not pass silently."""
    return [key for key, tally in tallies.items() if tally.compared == 0]


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

    empty_locales = [locale for locale in locales if not public_rulesets(locale)]
    if empty_locales:
        print(f"ICU lists no spellout rulesets for: {', '.join(empty_locales)}", file=sys.stderr)
        return 1

    drifted = False
    for locale in locales:
        only_icu, only_vendored = ruleset_drift(public_rulesets(locale), vendored_public_rulesets(locale))
        for name in only_icu:
            print(f"ruleset drift: locale={locale} {name} is listed by ICU but not vendored", file=sys.stderr)
        for name in only_vendored:
            print(f"ruleset drift: locale={locale} {name} is vendored but not listed by ICU", file=sys.stderr)
        drifted = drifted or bool(only_icu or only_vendored)
    if drifted:
        return 1

    requests: list[tuple[str, str, int]] = [
        (locale, ruleset, n)
        for locale in locales
        for ruleset in public_rulesets(locale)
        for n in values
    ]

    print(f"requesting {len(requests)} (locale, ruleset, n) tuples", file=sys.stderr)
    ours = run_rust(args.cargo_args.split(), requests)

    rbnf_cache: dict[str, "icu.RuleBasedNumberFormat"] = {}
    tallies, mismatch_lines = compare(
        requests, ours, lambda locale, ruleset, n: icu_spell(rbnf_cache, locale, ruleset, n)
    )
    for line in mismatch_lines:
        print(line)

    for (locale, ruleset), t in tallies.items():
        print(
            f"{locale} {ruleset}: compared {t.compared} of {t.requested}, "
            f"{t.icu_errors} ICU error(s), {t.mismatches} mismatch(es)",
            file=sys.stderr,
        )
    compared = sum(t.compared for t in tallies.values())
    icu_errors = sum(t.icu_errors for t in tallies.values())
    print(
        f"{len(mismatch_lines)} mismatch(es) in {compared} comparisons "
        f"({len(requests)} requested, {icu_errors} skipped on ICU error)",
        file=sys.stderr,
    )

    failed = False
    for locale, ruleset in vacuous(tallies):
        print(f"no comparison made for locale={locale} ruleset={ruleset}", file=sys.stderr)
        failed = True
    return 1 if failed or mismatch_lines else 0


if __name__ == "__main__":
    raise SystemExit(main())
