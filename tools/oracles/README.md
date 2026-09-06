# tools/oracles — second-opinion oracles

Runs the golden fixtures (`verbalize/tests/fixtures/<lang>/*.tsv`) through
every third-party normaliser available on this machine and prints the
lines where the oracle disagrees with our expected output, as
`class: input | expected | oracle`. It is a review aid:
disagreements are never failures, and the oracles are only executed,
never linked or copied.

```sh
python3 tools/oracles/oracles.py --lang de            # all oracles
python3 tools/oracles/oracles.py --lang de --oracles tpr,icu
```

| Oracle | What is compared | How it is found |
|---|---|---|
| `tpr` — [text-processing-rs](https://github.com/FluidInference/text-processing-rs) (Apache-2.0) | every fixture line, via `tn_normalize_sentence_lang` | built once from the git revision pinned in `tpr/Cargo.toml` into `.cache/` (gitignored); needs `cargo` |
| `num2words` (LGPL) | bare numbers in `cardinal.tsv`/`year.tsv` | Python module |
| `icu` — ICU4C RBNF via PyICU (Unicode) | bare numbers in `cardinal.tsv`/`year.tsv` | Python module `icu` |
| `nemo` — NeMo text processing (Apache-2.0) | every fixture line | Python module `nemo_text_processing`; skipped when absent |
| `espeak` | — | detected and skipped: espeak-ng emits phonemes, not words |

Comparison is loose: case, whitespace, hyphens, diacritics and the
`ä→ae`/`ß→ss` ASCII folding do not count as a disagreement, so what is
left is a genuine difference in reading. On NixOS:

```sh
nix shell --impure --expr 'let p = import <nixpkgs> {}; in p.python3.withPackages (ps: [ ps.pyicu ps.num2words ])' \
  --command python3 tools/oracles/oracles.py --lang de
```

`tpr/` is a standalone driver crate, deliberately outside the workspace.
