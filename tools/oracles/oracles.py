#!/usr/bin/env python3
"""Second-opinion oracles over the golden fixtures (design §11.4).

For every oracle available on this machine, spells each fixture input and
prints the lines where the oracle disagrees with our expected output as
`input | expected | oracle`. Disagreements are review material, never
failures: the exit code is 0 whenever the run completed.

Usage: python3 oracles.py [--lang de] [--oracles tpr,num2words,icu,nemo,espeak]
"""
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FIXTURES = ROOT / "verbalize" / "tests" / "fixtures"
CACHE = HERE / ".cache"
NUMBER = re.compile(r"^-?[0-9][0-9.,\s\u00a0\u202f\u2009]*$")


def fixtures(lang: str) -> list[tuple[str, str, str]]:
    """(file, input, expected) for every case of the language."""
    cases = []
    for path in sorted((FIXTURES / lang).glob("*.tsv")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#"):
                continue
            src, expected = line.split("\t", 1)
            cases.append((path.stem, src, expected))
    return cases


def loose(s: str) -> str:
    """Case, whitespace, hyphens and diacritics do not count as disagreement."""
    for a, b in (("ß", "ss"), ("ä", "ae"), ("ö", "oe"), ("ü", "ue"), ("Ä", "ae"), ("Ö", "oe"), ("Ü", "ue")):
        s = s.replace(a, b)
    s = unicodedata.normalize("NFKD", s)
    s = "".join(c for c in s if not unicodedata.combining(c))
    return re.sub(r"[\s\-‑–]+", "", s.casefold().rstrip(".,;:!?"))


def bare_number(src: str, lang: str) -> int | None:
    """The integer a bare numeric fixture input denotes, in the language's format."""
    if not NUMBER.match(src) or "," in src and lang != "en" and len(src.split(",")[-1]) != 3:
        return None
    digits = re.sub(r"[^0-9-]", "", src)
    if lang == "en" and "." in src:
        return None
    try:
        return int(digits)
    except ValueError:
        return None


# ----------------------------------------------------------------- oracles

def oracle_tpr(lang: str, cases: list[tuple[str, str, str]]) -> list[str | None] | str:
    """text-processing-rs, built once from its pinned git revision into .cache."""
    binary = CACHE / "target" / "release" / "tpr-oracle"
    if not binary.exists():
        if shutil.which("cargo") is None:
            return "cargo not found"
        subprocess.run(
            ["cargo", "build", "--release", "--quiet", "--manifest-path", str(HERE / "tpr" / "Cargo.toml"),
             "--target-dir", str(CACHE / "target")],
            check=True,
        )
    stdin = "".join(f"{lang}\t{src}\n" for _, src, _ in cases)
    run = subprocess.run([str(binary)], input=stdin, capture_output=True, text=True, check=True)
    return run.stdout.split("\n")[: len(cases)]


def oracle_num2words(lang: str, cases):
    """Python num2words on bare numbers of the cardinal/year/ordinal files."""
    try:
        from num2words import num2words
    except ImportError:
        return "python module num2words not installed"
    out = []
    for stem, src, _ in cases:
        n = bare_number(src, lang) if stem in ("cardinal", "year") else None
        try:
            out.append(None if n is None else num2words(n, lang=lang, to="year" if src.isdigit() else "cardinal"))
        except NotImplementedError as e:
            return f"num2words: {e}"
    return out


def oracle_icu(lang: str, cases):
    """ICU4C RBNF via PyICU on bare numbers of the cardinal/year files."""
    try:
        import icu
    except ImportError:
        return "python module icu (PyICU) not installed"
    rbnf = icu.RuleBasedNumberFormat(icu.URBNFRuleSetTag.SPELLOUT, icu.Locale(lang))
    out = []
    for stem, src, _ in cases:
        n = bare_number(src, lang) if stem in ("cardinal", "year") else None
        if n is None:
            out.append(None)
            continue
        rbnf.setDefaultRuleSet("%spellout-numbering-year" if src.isdigit() else "%spellout-numbering")
        f = icu.Formattable()
        f.setInt64(n)
        out.append(rbnf.format(f).replace("\u00ad", ""))
    return out


def oracle_nemo(lang: str, cases):
    try:
        from nemo_text_processing.text_normalization.normalize import Normalizer
    except ImportError:
        return "python module nemo_text_processing not installed"
    normalizer = Normalizer(input_case="cased", lang=lang)
    return [normalizer.normalize(src) for _, src, _ in cases]


def oracle_espeak(lang: str, cases):
    if shutil.which("espeak-ng") is None:
        return "espeak-ng not found"
    return "espeak-ng emits phonemes, not words; nothing to compare (skipped)"


ORACLES = {"tpr": oracle_tpr, "num2words": oracle_num2words, "icu": oracle_icu, "nemo": oracle_nemo, "espeak": oracle_espeak}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lang", default="de")
    parser.add_argument("--oracles", default=",".join(ORACLES))
    args = parser.parse_args()
    cases = fixtures(args.lang)
    print(f"{len(cases)} fixture lines for {args.lang}")
    for name in args.oracles.split(","):
        result = ORACLES[name](args.lang, cases)
        if isinstance(result, str):
            print(f"\n== {name}: skipped — {result}")
            continue
        compared = [(c, o) for c, o in zip(cases, result) if o is not None]
        disagreements = [(c, o) for c, o in compared if loose(o) != loose(c[2])]
        print(f"\n== {name}: {len(compared)} compared, {len(disagreements)} disagree")
        by_file: dict[str, int] = {}
        for (stem, src, expected), out in disagreements:
            by_file[stem] = by_file.get(stem, 0) + 1
            print(f"{stem}: {src} | {expected} | {out.strip()}")
        if by_file:
            print("per file: " + ", ".join(f"{k}={v}" for k, v in sorted(by_file.items(), key=lambda kv: -kv[1])))
    return 0


if __name__ == "__main__":
    sys.exit(main())
