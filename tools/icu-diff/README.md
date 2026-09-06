# `tools/icu-diff`

Differential test of the verbalize RBNF core against ICU4C, via PyICU
(design §11.2). Not linked into the crate; PyICU (LGPL) is safe here
because it is only ever *run*, per §6.4.

## What it does

`icu_diff.py` enumerates every public spellout ruleset — via
`RuleBasedNumberFormat.getRuleSetName()` — for each vendored locale
(`de`, `en`, `ro`), builds a shared sample of integers (the full range
0–10,000, a configurable stride over 0–1,000,000, powers of ten to 10^18,
and 10,000 seeded random values), spells every `(locale, ruleset, n)`
triple with PyICU, and compares it against the same triple spelled by
`verbalize/examples/rbnf_dump.rs` (built and run via `cargo run --release`).
Both sides are normalised identically before comparing: CLDR's soft
hyphens (U+00AD) are stripped and whitespace is collapsed, matching the
renderer's own post-processing (design §8).

Any mismatch is printed as `MISMATCH locale=... ruleset=... n=... icu=...
ours=...` on stdout; the process exits non-zero if there is at least one.

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

That example is written against an entry point that does not exist yet:

```rust
pub fn verbalize::spell::ruleset(language: Language, ruleset: &str, n: i128) -> Option<String>;
```

`ruleset` is the exact CLDR ruleset name (public or `%%` private); `None`
for an unknown name. `rbnf` itself stays private, per the design's
workspace layout (§4) — this is a `spell`-module entry point, not a new
public `rbnf` module. Once it lands, the example compiles unchanged and
this tool works end to end. (The rest of the public API in design §5,
`verbalize::spell::{cardinal, ordinal, year, digits}`, is a different,
higher-level surface for callers who already know the semiotic class;
`ruleset` is the raw ruleset-by-name entry point the differential test
needs to reach every CLDR ruleset, including ones the higher-level
functions never call directly. Only integers are compared — ICU's
`RuleBasedNumberFormat` holds a `double`/`int64` internally, so fractional
input isn't comparable through this path anyway.)

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

On NixOS (no system-wide `libicu-dev`), point `pkg-config` and the runtime
loader at a built `icu4c` derivation instead, e.g.:

```sh
export PKG_CONFIG_PATH="$(nix eval --raw nixpkgs#icu74.dev)/lib/pkgconfig:$PKG_CONFIG_PATH"
uv pip install --python .venv/bin/python -r requirements.txt
export LD_LIBRARY_PATH="$(nix eval --raw nixpkgs#icu74)/lib:$LD_LIBRARY_PATH"
```

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
(design §11.2) and fails the build on any mismatch.

## Environment checked (2026-09-06)

`python3 -c 'import icu'` fails out of the box on this NixOS machine: there
is no `libicu-dev`-equivalent on `PKG_CONFIG_PATH` by default, so `pip
install PyICU` cannot find ICU's headers/pkg-config files. A prebuilt
`icu4c-76.1` (with a `-dev` output carrying `icu-i18n.pc`/`icu-uc.pc`) was
already present in the local Nix store as a transitive dependency of
another package; pointing `PKG_CONFIG_PATH` at its `-dev` output's
`lib/pkgconfig` and `LD_LIBRARY_PATH` at its `lib` let `uv pip install
PyICU` build and import successfully against **ICU 76.1**
(`icu.ICU_VERSION == "76.1"`). This is host-specific — a fresh clone
should follow the Debian/Fedora/Homebrew/Nix instructions above rather
than rely on a store path found by chance.
