# verbalize

Text normalisation for speech synthesis: turns text written for the eye
("8,80 €", "19.30 Uhr", "1. November") into text written for the ear
("acht Euro achtzig", "neunzehn Uhr dreißig", "erster November"), driven by
vendored CLDR data, in a language-neutral way. German, English and
Romanian are supported.

```rust
use verbalize::{Language, Normalizer};

let de = Normalizer::new(Language::De);
assert_eq!(
    de.normalize("Am 1. November kostet es 8,80 €."),
    "Am ersten November kostet es acht Euro achtzig."
);
```

(Verified: `echo "Am 1. November kostet es 8,80 €." | cargo run -q -p verbalize-cli -- normalize` prints exactly that.)

## Languages

| Language | Status | Example |
|---|---|---|
| German (`de`) | Complete: every class below, case/gender agreement, 545 fixture lines, `docs/rules/de.md` | `Am 1. November kostet es 8,80 €.` → `Am ersten November kostet es acht Euro achtzig.` |
| English (`en`) | Complete: every class, `Options::region` `EnUs` (default) / `EnGb` for date order, 383 fixture lines, `docs/rules/en.md` | `On November 1, 2026 I paid $8.80 at 2:30 pm.` → `On November first, twenty twenty-six I paid eight dollars and eighty cents at two thirty p m.` |
| Romanian (`ro`) | Complete: every class, gender agreement and the "de" construction, 344 fixture lines, `docs/rules/ro.md` | `Pe 1 noiembrie 2026 am plătit 20 de lei la ora 14:30.` → `Pe întâi noiembrie două mii douăzeci și șase am plătit douăzeci de lei la paisprezece și treizeci.` |

The three modules share one core (`verbalize/src/lang/common.rs`): the
language-neutral recognisers, the CLDR unit resolver and the number
format are parametrised by a per-language `Context` of tables.

## Semiotic classes

One example per class and language, the first line of the matching file in
`verbalize/tests/fixtures/<lang>/` (input → expected output verified by
`cargo test -p verbalize --test fixtures`):

| Class | German | English | Romanian |
|---|---|---|---|
| Cardinal | `12` → `zwölf` | `12` → `twelve` | `12` → `doisprezece` |
| Year | `1990` → `neunzehnhundertneunzig` | `1990` → `nineteen ninety` | `1990` → `o mie nouă sute nouăzeci` |
| Ordinal | `3. Klasse` → `dritte Klasse` | `1st` → `first` | `al 3-lea` → `al treilea` |
| Decimal | `3,5` → `drei Komma fünf` | `3.5` → `three point five` | `3,5` → `trei virgulă cinci` |
| Dotted number | `2.3` → `zwei Punkt drei` | `version 1.2.7` → `version one point two point seven` | `2.3` → `doi punct trei` |
| Money | `8,80 €` → `acht Euro achtzig` | `$8.80` → `eight dollars and eighty cents` | `1 leu` → `un leu` |
| Percent / permille | `12 %` → `zwölf Prozent` | `12 %` → `twelve percent` | `12 %` → `doisprezece la sută` |
| Degrees | `20 °C` → `zwanzig Grad Celsius` | `68 °F` → `sixty-eight degrees Fahrenheit` | `20 °C` → `douăzeci de grade Celsius` |
| Measure | `10 km` → `zehn Kilometer` | `10 km` → `ten kilometers` | `10 km` → `zece kilometri` |
| Time | `14:30` → `vierzehn Uhr dreißig` | `9:00` → `nine o'clock` | `14:30` → `paisprezece și treizeci` |
| Time range | `14:00–16:00` → `vierzehn Uhr bis sechzehn Uhr` | `9:00–11:00` → `nine o'clock to eleven o'clock` | `9:00–11:00` → `ora nouă până la ora unsprezece` |
| Date | `1.11.2026` → `erster November zweitausendsechsundzwanzig` | `11/01/2026` → `November first twenty twenty-six` | `1 noiembrie 2026` → `întâi noiembrie două mii douăzeci și șase` |
| Range | `5–10 Minuten` → `fünf bis zehn Minuten` | `5–10 minutes` → `five to ten minutes` | `5–10 minute` → `cinci până la zece minute` |
| Score | `2:1` → `zwei zu eins` | `2:1` → `two to one` | `2:1` → `doi la unu` |
| Telephone | `01234/56789` → `null eins zwei drei vier, fünf sechs sieben acht neun` | `+1 (555) 123-4567` → `plus one, five five five, one two three, four five six seven` | `+40 21 123 4567` → `plus patru zero, doi unu, unu doi trei, patru cinci șase șapte` |
| Long digit run | `007` → `null null sieben` | `007` → `zero zero seven` | `007` → `zero zero șapte` |
| Abbreviation | `Abs.` → `Absatz` | `e.g.` → `for example` | `etc.` → `etcetera` |
| Unicode fraction | `½` → `ein halb` | `½` → `one half` | `½` → `o jumătate` |
| Roman numeral (clinical) | `NYHA II-III` → `NYHA zwei bis drei` | `NYHA II-III` → `NYHA two to three` | `NYHA II-III` → `NYHA doi până la trei` |
| Scientific notation | `1,5 × 10⁻⁶` → `eins Komma fünf mal zehn hoch minus sechs` | `1.5 × 10⁻⁶` → `one point five times ten to the power of minus six` | `1,5 × 10⁻⁶` → `unu virgulă cinci ori zece la puterea minus șase` |
| Power / exponent | `2⁵` → `zwei hoch fünf` | `2⁵` → `two to the power of five` | `2⁵` → `doi la puterea cinci` |
| Chemical formula | `H₂O` → `H zwei O` | `H₂O` → `H two O` | `H₂O` → `H doi O` |
| Math expression | `3 + 4 = 7` → `drei plus vier gleich sieben` | `3 + 4 = 7` → `three plus four equals seven` | `3 + 4 = 7` → `trei plus patru egal șapte` |
| Slash fraction | `1/2` → `ein halb` | `1/2` → `one half` | `1/2` → `o jumătate` |
| Ratio / titer / blood pressure | `120/80 mmHg` → `einhundertzwanzig zu achtzig Millimeter Quecksilbersäule` | `120/80 mmHg` → `one hundred twenty over eighty millimeters of mercury` | `120/80 mmHg` → `o sută douăzeci cu optzeci de milimetri coloană de mercur` |
| Dose scheme | `20-5-0 mg/d` → `zwanzig, fünf, null Milligramm pro Tag` | `1-0-1` → `one, zero, one` | `1-0-1` → `unu, zero, unu` |
| Repetition / multiplication | `2x täglich` → `zweimal täglich` | `2x daily` → `twice daily` | `2x pe zi` → `de două ori pe zi` |
| Angle / coordinate | `30°15′` → `dreißig Grad fünfzehn Minuten` | `30°15′` → `thirty degrees fifteen minutes` | `30°15′` → `treizeci de grade cincisprezece minute` |
| Electronic | `info@firma.de` → `info at firma Punkt de` | `a.bc@gmail.com` → `a dot bc at gmail dot com` | `info@firma.ro` → `info at firma punct ro` |
| Paragraph sign | `§ 25` → `Paragraf fünfundzwanzig` | `§ 25` → `section twenty-five` | `§ 25` → `paragraful douăzeci și cinci` |

Full written/spoken conventions, agreement rules and sources: per-rule
citations in `docs/rules/{de,en,ro}.md`.

## CLI

```sh
cargo install verbalize-cli          # from crates.io; installs the `verbalize` binary
cargo install --path verbalize-cli   # from a checkout
```

Prebuilt binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64) and
Windows (x86_64) ship with every [GitHub release](https://github.com/relu/verbalize/releases),
with shell/PowerShell installers.

```sh
# Normalize whole input once (not per line), stdin or a file
echo "Am 1. November um 19.30 Uhr kostet es 8,80 €." | verbalize normalize
# Am ersten November um neunzehn Uhr dreißig kostet es acht Euro achtzig.

# One classified span per line: byte_start-byte_end, class, original, spoken, fallback
echo "Am 1. November um 19.30 Uhr kostet es 8,80 €." | verbalize annotate
# 3-14	Date	1. November	ersten November	false
# 18-27	Time	19.30 Uhr	neunzehn Uhr dreißig	false
# 38-46	Money	8,80 €	acht Euro achtzig	false

verbalize annotate --json input.txt      # same spans as a JSON array

# Coverage: extract every digit/symbol shape from a corpus, grouped by
# frequency, plus the shapes that are still unhandled (fallback or verbatim)
verbalize survey corpus/ --threshold 5   # non-zero exit if an unhandled
                                          # shape occurs 5+ times (for CI)
verbalize survey sqlite:app.db           # every TEXT column, every table
```

Flags shared by `normalize`/`annotate`: `--lang de|en|ro` (default `de`),
`--region <tag>` (e.g. `de-CH`, `en-US`), `--no-abbreviations`, `--lexicon
FILE` (see below), and a positional `FILE` or `-`/omitted for stdin.

## `Options`

```rust
pub struct Options {
    pub expand_abbreviations: bool,   // "z. B." → "zum Beispiel"; default true
    pub region: Option<Region>,       // de-CH "Rappen", en-US vs en-GB dates, …
    pub lexicon: Vec<(String, String)>, // caller domain shorthand, applied last
}
```

`lexicon` is a literal, case-sensitive, word-bounded substitution applied
after every digit-bearing class, so it can add domain vocabulary (clinical
shorthand, product codes, …) without ever shadowing a phone number, dose or
ratio span. The CLI reads it from a TSV file, `literal<TAB>spoken` per line:

```
SpO2	Sauerstoffsättigung
```

```sh
$ echo "SpO2 92 %" | verbalize normalize --lexicon lexicon.tsv
Sauerstoffsättigung zweiundneunzig Prozent
```

(Verified: both commands above were run as shown.)

## Data: CLDR

All number spelling, currency/unit/month/weekday names and plural rules
come from vendored CLDR JSON (`unicode-org/cldr-json`), pinned in
`xtask/cldr.lock` (currently `48.2.1`) and compiled into
`verbalize/src/data/` — both the raw JSON and the generated Rust are
checked in, so a clone builds offline.

```sh
cargo xtask cldr-update <tag>   # download the pinned files for de/en/ro
cargo xtask gen                 # compile them into verbalize/src/data/
cargo xtask check-data          # CI guard: fails if generated data is stale
```

## Testing

- **Golden fixtures** — `verbalize/tests/fixtures/de/<class>.tsv`
  (`input<TAB>expected`), one file per semiotic class plus `mixed.tsv` for
  interactions; run with `cargo test -p verbalize --test fixtures`.
- **Property tests** — idempotence (`normalize(normalize(t)) == normalize(t)`)
  and "output contains no ASCII digits", over every fixture input and over
  random digit-rich strings.
- **`tools/icu-diff`** — differential test of the RBNF interpreter against
  ICU4C via PyICU: spells 0–1,000,000 (strided), powers of ten to 10¹⁸, and
  10,000 random values for every vendored locale and public ruleset. Any
  mismatch is a failure; see `tools/icu-diff/README.md`.

## Releasing

Releases are cut by [cargo-dist](https://opensource.axo.dev/cargo-dist/)
from `.github/workflows/release.yml` (generated; edit
`dist-workspace.toml` and re-run `dist generate`, never the YAML). Both
crates share one version and one tag.

1. Bump `version` in `verbalize/Cargo.toml` and `verbalize-cli/Cargo.toml`
   (and the `verbalize` dependency in the latter); move the `[Unreleased]`
   notes in `CHANGELOG.md` under a dated `## [x.y.z]` heading.
2. `dist plan` shows what the tag will produce; commit.
3. `git tag vx.y.z && git push --tags`.

The tag triggers the workflow: it builds the CLI for every target, runs
`.github/workflows/publish-crates.yml` (`cargo publish` for `verbalize`
then `verbalize-cli`, authenticated by crates.io Trusted Publishing — each
crate lists `relu/verbalize` / `release.yml` as a trusted publisher, no
token secret), then creates the GitHub release with the archives,
installers, checksums and the changelog section for that version as
release notes.

## Licence

Dual-licensed [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE). Vendored
CLDR data is under the Unicode License v3 (SPDX `Unicode-3.0`); see
[`NOTICE`](NOTICE) for the full text and provenance.

## Rule provenance

Every hand-written rule (written-form conventions, agreement, per-language
tables) cites the standard and edition it was checked against
(DIN 5008:2020, Duden 28th ed., ISO 6709, clinical documentation
conventions, …): `docs/rules/<lang>.md`.
