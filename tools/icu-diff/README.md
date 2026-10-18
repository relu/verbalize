# `tools/icu-diff`

Differential test of the verbalize RBNF core against ICU4C, via PyICU.
Not linked into the crate; PyICU (LGPL) is safe here because it is only
ever *run*.

## What it does

`icu_diff.py` enumerates every public spellout ruleset — via
`RuleBasedNumberFormat.getRuleSetName()` — for each vendored locale
(`de`, `en`, `ro`), builds a shared sample of integers (the full range
0–10,000 and its negatives, so the `-x` rules are compared too; a
configurable stride over 0–1,000,000; powers of ten and their negatives to
10^18; and 10,000 seeded random values, a fifth of them negative), spells
every `(locale, ruleset, n)`
triple with PyICU, and compares it against the same triple spelled by
`verbalize/examples/rbnf_dump.rs` (built and run via `cargo run --release`).
Both sides are normalised identically before comparing: CLDR's soft
hyphens (U+00AD) are stripped and whitespace is collapsed, matching the
renderer's own post-processing.

Any mismatch is printed as `MISMATCH locale=... ruleset=... n=... icu=...
ours=...` on stdout. The run exits non-zero if there is at least one
mismatch, or if any ruleset ends with zero comparisons made (a ruleset
renamed or dropped by ICU would otherwise pass vacuously), or if ICU lists
no spellout rulesets for a locale.

A value ICU cannot spell (it throws `ICUError`) is skipped rather than
compared. Those skips are counted per `(locale, ruleset)` and printed on
stderr next to the number of comparisons actually made, as in
`de %spellout-numbering: compared 31022 of 31022, 0 ICU error(s), 0
mismatch(es)`. The final summary line reports comparisons made separately
from tuples requested.

**Precision note**: the script formats every value through
`icu.Formattable().setInt64(n)`, not `rbnf.format(n)` directly. PyICU's
plain-integer overload silently round-trips through a C `double`, which
loses precision above 2^53 (~9.007×10^15) — e.g. it renders both
`999999999999999999` and `10**18` identically. Routing through an explicit
`Formattable` with `setInt64` keeps full `int64_t` precision, which covers
the whole 10^18 range this tool tests.

## The Rust side of the contract

`icu_diff.py` never calls into `verbalize` directly. It shells out to a
small example binary, `verbalize/examples/rbnf_dump.rs`, that reads
`<locale>\t<ruleset>\t<n>` lines from stdin and prints the spelled form on
stdout, one line per request, or the sentinel `<NONE>` for a request the
interpreter has no answer for.

That example calls one entry point:

```rust
pub fn verbalize::spell::ruleset(language: Language, ruleset: &str, n: i128) -> Option<String>;
```

`ruleset` is the exact CLDR ruleset name (public or `%%` private); `None`
for an unknown name. `rbnf` itself stays private — this is a
`spell`-module entry point, not a public `rbnf` module. The rest of the
`spell` API (`cardinal`, `ordinal`, `year`, `digits`) is a higher-level
surface for callers who already know the semiotic class; `ruleset` is the
raw ruleset-by-name entry point the differential test needs to reach every
CLDR ruleset, including ones the higher-level functions never call
directly. Only integers are compared — ICU's `RuleBasedNumberFormat` holds
a `double`/`int64` internally, so fractional input isn't comparable through
this path anyway.

## Install

PyICU binds against your system's ICU4C — there is no manylinux wheel with
a bundled ICU, so a working `libicu` (with headers, i.e. the `-dev`
package) must be discoverable via `pkg-config` (or the `ICU_VERSION`
environment variable) before `pip install PyICU` will build.

The ICU you bind against must carry the same CLDR generation as the
vendored data (`xtask/cldr.lock`, CLDR 48 → ICU 78). Older ICUs diff
against older CLDR: Ubuntu 24.04's libicu 74 (CLDR 44) still spells
Romanian with cedilla `ş`, which CLDR 46+ replaced with comma-below `ș`,
so every `ro` tuple mismatches. CI therefore installs the official
`icu4c-78.3-Ubuntu22.04-x64.tgz` build rather than the distro package.

```sh
# Debian/Ubuntu
sudo apt-get install libicu-dev pkg-config

# Fedora
sudo dnf install libicu-devel pkgconf-pkg-config

# macOS (Homebrew keeps icu4c unlinked; point pkg-config at it)
brew install icu4c
export PKG_CONFIG_PATH="$(brew --prefix icu4c)/lib/pkgconfig:$PKG_CONFIG_PATH"

# then, from tools/icu-diff/
uv venv .venv
uv pip install --python .venv/bin/python -r requirements.txt
```

On NixOS (no system-wide `libicu-dev`), take ICU and PyICU from nixpkgs
instead of pip. With the nixpkgs revision this was checked against, the
first command prints `78.3 2.16.2`:

```sh
nix shell --impure --expr 'with import <nixpkgs> {}; python3.withPackages (p: [p.pyicu])' \
  -c python3 -c 'import icu; print(icu.ICU_VERSION, icu.VERSION)'   # 78.3 2.16.2
nix shell --impure --expr 'with import <nixpkgs> {}; python3.withPackages (p: [p.pyicu])' \
  -c python3 tools/icu-diff/icu_diff.py
```

`requirements.txt` pins `PyICU==2.16.2`: 2.16 added ICU 78.1 support and
2.16.1 added 78.2, and 2.16.2 is the release nixpkgs builds against
ICU 78.3, the version CI installs. Releases before 2.16 predate ICU 78.
When CI moves to another ICU, check the PyICU changelog before bumping
the pin.

The `.venv/` directory is local and gitignored; it is never committed.

## Usage

```sh
export LD_LIBRARY_PATH="<path to the icu4c lib/ used at install time>:$LD_LIBRARY_PATH"
.venv/bin/python icu_diff.py                       # de,en,ro; default stride and seed
.venv/bin/python icu_diff.py --locales de --stride 10007
.venv/bin/python icu_diff.py --cargo-args "run --release -q -p verbalize --example rbnf_dump"
```

Run from the repository root, or set `--cargo-args` to whatever invocation
finds the workspace `Cargo.toml`. CI runs this in a job that installs ICU
and fails the build on any mismatch.
