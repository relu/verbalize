# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Minimum supported Rust version declared: `rust-version = "1.88"` (set
  by the `icu_*` 2.3 dependencies), checked by an `msrv` CI job.
- Tests: every fixture is also read inside a sentence (a word, a line
  break, parentheses, a comma or a period around it) and must leave the
  surrounding text byte-for-byte intact; spans may not start or end with
  whitespace or cross a line break; the random-input tests include
  non-breaking and other Unicode spaces.

### Changed

- **Breaking:** `Language` is `#[non_exhaustive]`; a downstream `match`
  needs a wildcard arm, so adding a language is no longer breaking.
- **Breaking:** the generated CLDR tables (`verbalize::data`) are no
  longer public, and the doc-hidden `check_rbnf` is removed.
- `Options` and `Agreement` are documented as struct literals built with
  `..Default::default()`, since a 0.x minor release may add fields.
- Idempotence is no longer promised for every string: a reading can
  expose a token the first pass saw only glued into a larger one
  (`www.example.com/Nr.` → `… Nr.` → `… Nummer`). It holds for every
  fixture input and the random digit-rich strings of the property tests.
- Release workflow: the GitHub release is created only after `cargo
  publish` succeeds, so a failed publish no longer leaves a release for a
  version that is not on crates.io; re-running the job skips crates
  already published. Every action in `release.yml` is pinned to a commit
  SHA.
- `cargo xtask cldr-update` swaps the new data in only after every file
  downloaded, so a failed download no longer deletes `xtask/cldr`;
  `gen`/`check-data` fail when the vendored CLDR version differs from
  `xtask/cldr.lock`. `xtask` no longer depends on `verbalize`.
- `tools/icu-diff` compares negative values too, fails when ICU's public
  rulesets differ from the shipped ones or a ruleset gets no comparisons,
  reports comparisons made, and pins `PyICU==2.16.2`.

### Fixed

- A degree value or dose scheme followed by a non-breaking or other
  multi-byte space and a letter (`270°` + U+00A0 + `Nord`, `1-0-1` +
  U+00A0 + `mg`) panicked. A chemical count beyond `u32`
  (`H5000000000O`) panicked in debug builds; it now stays as written.
  `de`/`en`/`ro`.
- Text after a reading is kept intact: `Es hat 20° in Berlin` was
  "zwanzig Gradin Berlin", `1,5 × 10³ Zellen` was "… hoch dreiZellen";
  numbers, units, signs and abbreviations are never joined across a line
  break, so `Pagina 21` at the end of a line no longer takes "de" from
  the next line, and a phone number keeps the parentheses around it. Two
  readings that touch are separated: `Nr.2023` was
  "Nummerzweitausenddreiundzwanzig", `ca.5 kg` "zirkafünf Kilogramm".
  `de`/`en`/`ro`.
- A unit after a comparison sign or scale word is read when more text
  follows (`≤10 mg pro Tag` → "kleiner oder gleich zehn Milligramm pro
  Tag"), and `2x.`, `3-mal,`, `∞.`, `30 €/Monat.` are read before
  punctuation. `(§)` is "(Paragraf)" with no stray space. `de`/`en`/`ro`.
- Negative amounts with a scale word or three or more decimals said
  "minus" twice (`-2,5 Mrd. €`, `USD -5.125`); `-1 Mio.` / `-1 mil.`
  take the singular scale word. `de`/`en`/`ro`.
- German: `um 1 Uhr` is "um ein Uhr" (was "eins Uhr"). After an article
  or preposition, `N.` before a lower-case adjective is an ordinal with
  the adjective's ending (`der 3. große Sieg` → "der dritte große Sieg",
  `nach 2. langer Pause` → "nach zweiter langer Pause"). An angle of one
  minute or degree is singular (`30°1'` → "dreißig Grad eine Minute",
  "one minute"). `de`/`en`.
- Romanian: `1 ¼` is "unu și un sfert" (was "unu și sfert"); `al II-lea5`
  reads the ordinal and the number as separate words. `ro`.
- Plural forms and ordinal suffixes of numbers above 2⁶⁴ follow the full
  value instead of one clamped to `u64::MAX`. `en`/`ro`.
- CLI: piping into a command that exits early (`… | head -1`) no longer
  panics; a failing `--threshold` still exits 1. `sqlite:` paths open
  read-only, so a mistyped path is an error instead of an empty, passing
  database; columns are chosen by SQLite's TEXT affinity (`VARCHAR`),
  BLOB/NULL values are skipped, and tables like `sqlitex` are scanned.
  Symlinked directories are not followed (no loops). `annotate` escapes
  tab, newline, carriage return and backslash in its TSV output.
  `--region` must belong to `--lang`.
- `tools/corpus/check.sh --strict` fails when no corpus (or only an empty
  directory) is present instead of passing having checked nothing.
- Docs: `Normalizer::with_options` no longer says English and Romanian
  return the input verbatim; the README `normalize` example passes
  `--lang de`.

## [0.1.5] - 2026-09-20

### Added

- `Options::region` now changes German readings: `DeAt` (Austria) spells
  and recognises January as "Jänner"/"Jän." instead of "Januar"/"Jan."
  (Duden, Österreichisches Wörterbuch), consistently across the Date
  reading and the abbreviation expansion; `DeCh` (Switzerland) has no "ß"
  (Bundeskanzlei "Weisungen zur Rechtschreibung") — every generated
  spoken form uses "ss" instead ("dreißig" → "dreissig"). Previously
  these regions were accepted but read identically to `DeDe`.

### Fixed

- README: the `--lexicon` example produced English, not the documented
  German output, after the 0.1.2 default-`--lang` change; now passes
  `--lang de` explicitly.

## [0.1.4] - 2026-09-20

### Fixed

- Units: German `dem` is the article, not `d` + `em` (deci-em); `2007 dem`
  was "zweitausendsieben Deziem". German `in` is a preposition, not the
  inch symbol; `5 in der Woche` was "fünf Zoll der Woche". Both are denied
  after a bare number. Found by the corpus audit over the ECDC medical
  sample. `de`.
- Units: cubic centimetre reads as itself, not as an SI-prefix split of a
  shorter symbol — `cc` (`en`), `ccm` (`de`), `cc`/`ccm` (`ro`) were
  "centicenturies"/"Zentizentimeter"/"centicentimetri". Found by the new
  corpus audit (`verbalize audit`) over ~500k sentences.
- Units: Romanian `din` is a preposition, not `d` + `in` (deci-inch);
  `1 din 10` was "unu deciinch zece". English `my` is not `m` + `y`
  (milli-year). Both are now bare-token denials. `en`/`ro`.
- Units: the "a written-out unit name wins over an SI-prefix split" rule is
  now fed by every name a language declares — CLDR long forms, `long_name`,
  and the extra table's names — instead of only CLDR long patterns, and a
  test fails on any name that would still be split. The extra table's names
  also carry their gender now: `1 Kiloeinheit` is "eine Kiloeinheit", not
  "eins Kiloeinheit". `de`/`en`/`ro`.

## [0.1.3] - 2026-09-18

### Fixed

- Units: a written-out unit name after a number is no longer read as an
  SI prefix glued to a shorter symbol. German `29 Grad` was
  "neunundzwanzig Gigaradiant" (`G` + `rad`), now "neunundzwanzig Grad".
  `de`/`en`/`ro`.

## [0.1.2] - 2026-09-06

### Fixed

- Range: a unit right after a range (`5-10 Min.`, `1-2 kg`) is now
  claimed as part of the range instead of Measure claiming only the
  second number and stranding the first as a bare cardinal with the
  dash left unread (`5-10 Min.` was "fünf-zehn Minuten", now "fünf bis
  zehn Minuten"). `de`/`en`/`ro`.

### Changed

- CLI: the default `--lang` is now `en` (was `de`).

## [0.1.1] - 2026-09-06

### Fixed

- Money: an ISO 4217 currency code before the amount (`EUR 22 000`,
  invoice/statement style) is now recognized; previously only a bare
  symbol (`€`) worked as a prefix, so the code was left as stray text
  and the amount read as a plain cardinal. `de`/`en`/`ro`.

## [0.1.0] - 2026-09-06

First release.

### Added

- `Normalizer` (`normalize`, `annotate`) with `Options` (region,
  abbreviation expansion, caller lexicon) and the `spell` module
  (`cardinal`, `ordinal`, `year`, `digits`).
- CLDR/ICU rule-based number formatting (RBNF) interpreter, verified
  against ICU4C over every public spellout ruleset of the vendored locales
  (`tools/icu-diff`, 0 mismatches in 546,208 tuples).
- German (`de`): every semiotic class with case/gender agreement —
  cardinal, year, ordinal, decimal, money,
  percent, degrees, measure, time, time range, date, range, score,
  telephone, long digit runs, abbreviations, Unicode and slash fractions,
  clinical Roman numerals, scientific notation, powers, chemical formulae,
  math expressions, ratios/titers/blood pressure, dose schemes,
  repetition, angles/coordinates, paragraph signs; `docs/rules/de.md`.
- Dotted number class (`Token::Dotted`): section and version numbers
  (`2.3`, `3.10`, `1.2.7`) read group by group with "Punkt"/"point"/
  "punct" between the groups (`3.10` → "drei Punkt zehn"). Grouped
  cardinals (`1.000`), decimals (`2.3` in English) and dates are
  unaffected.
- `Token::class()` returns the semiotic class name, used for the CLI's
  `annotate` column and JSON field.
- Digital units: `KB`, `Mbit`, `Gbit`, `kbit`, `Tbit`, `bit`/`Bit`,
  `byte`/`Byte` aliases in all three languages and the `bps` spellings
  (`100 Mbps` → "einhundert Megabit pro Sekunde"); `GB`, `MB`, `TB`, `kB`
  already came from CLDR.
- Time zones: a zone name after a time stays as written (`14:30 CET`,
  `2:30 PM EST`; fixtures), and a `UTC`/`GMT` offset reads as an expression
  (`12:00 UTC+2` → "zwölf Uhr UTC plus zwei"; `MathItem::Word`).
- Digit groups (`Token::DigitGroups`): a 16-digit card number
  (`4111 1111 1111 1111`, all languages) and a US SSN (`123-45-6789`, en)
  read digit by digit with a pause between groups.
- Scale words with money and counts (`Token::Scale`, `Money::scale`,
  `Token::Scaled`): `2,5 Mrd. €` → "zwei Komma fünf Milliarden Euro",
  `$2.5 billion` → "two point five billion dollars", `3 mil. lei` → "trei
  milioane de lei", `1,2 Mio. Einwohner` → "eins Komma zwei Millionen
  Einwohner".
- Electronic class (`Token::Electronic`, highest priority):
  emails, `scheme://` URLs, `www.` and bare domains with a known TLD, IPv4
  addresses and `@handles` read symbol by symbol with per-language names
  (`@` at, `.` Punkt/dot/punct, `/` Schrägstrich/slash/bară, …), letters
  as written, digits one by one; `electronic.tsv` fixtures per language.
- Slash fractions with denominators 13–99 (`1/13` → "ein Dreizehntel",
  `31/32` → "thirty-one thirty-seconds", `3/20` → "trei douăzecimi"); en
  `1/4th` with the ordinal suffix.
- English: `YYYY/M/D` dates, `H:MM:SS` as counted hours, minutes and
  seconds, `$.01` amounts, email addresses with a path. German: a scale
  word before a unit (`2 Millionen km/h` → "zwei Millionen Kilometer pro
  Stunde"), also lower-case. Found by the second-opinion oracle run.
- ISO year-month (`2003-03`) reads as a Date ("März zweitausenddrei",
  "March two thousand three", "martie două mii trei"); `Token::Date::day`
  is `Option<u8>`.
- English (`en`) with `en-US`/`en-GB` date order; `docs/rules/en.md`.
- Romanian (`ro`) with gender agreement and the "de" construction;
  `docs/rules/ro.md`.
- Vendored CLDR 48.2.1 tables generated by `cargo xtask gen` (Unicode
  License v3, see `NOTICE`); `cargo xtask check-data` guards staleness.
- `verbalize` CLI: `normalize`, `annotate` (`--json`), `survey`; prebuilt
  binaries for Linux, macOS and Windows with shell/PowerShell installers
  on every GitHub release (cargo-dist).
- Golden fixtures per class and language (`verbalize/tests/fixtures/`),
  idempotence and no-digits-in-output property tests, a criterion bench,
  and `tools/oracles/` (text-processing-rs, num2words, PyICU, NeMo) as
  second-opinion review aids.
