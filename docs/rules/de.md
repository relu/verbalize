# German (`de`) — rule provenance

One section per semiotic class from the design's §7.2 table, in the same
order. Each section states the written-form rule, the spoken-form rule,
agreement notes where they apply, and the source it was checked against.

Written-form sources: DIN 5008:2020 (Schreib- und Gestaltungsregeln), Duden
— Die deutsche Rechtschreibung (28th ed.), CLDR de. Spoken-form sources:
CLDR de RBNF, Duden — Die Grammatik, Duden — Richtiges und gutes Deutsch.

## Cardinal

- **Written**: digit groups separated by `.`, a regular space, NBSP
  (U+00A0), NNBSP (U+202F), or a thin space (U+2009) — DIN 5008:2020 §5.
- **Spoken**: CLDR de RBNF `%spellout-numbering`.
- **Agreement**: "eins" standalone. "ein"/"eine" before a noun of known
  gender (currency, unit, or the ordinal noun lexicon) via
  `%spellout-cardinal-masculine`/`-feminine`/`-neuter`; unknown-gender
  nouns leave the bare "eins" reading (§7.3 of the design).
- **Source**: DIN 5008:2020; CLDR de `rbnf.json`
  (`spellout-numbering`, `spellout-cardinal-masculine`,
  `spellout-cardinal-feminine`, `spellout-cardinal-neuter`).

## Year

- **Written**: bare 4-digit number (`1990`, `2024`); decade form with an
  attached `er` (`1990er`) — Duden 28th ed. §718.
- **Spoken**: CLDR de RBNF `%spellout-numbering-year`. Grouped form
  (`1.990`) is not a year — DIN 5008 grouping marks it a Cardinal, read via
  `%spellout-numbering` instead, which differs from the year reading for
  the 1100–1999 range (hundred-based split vs. thousand-based split).
- **Source**: Duden 28th ed. §718; CLDR de `rbnf.json`
  (`spellout-numbering-year`).

## Ordinal

- **Written**: digit(s) followed by a period (`3.`, `1.`) — DIN 5008:2020
  §5.4.
- **Spoken**: CLDR de RBNF `%spellout-ordinal` with the ending selected by
  agreement (§7.3 below): `-r`/`-n`/`` /`-s`/`-m` rulesets.
- **Agreement**: only in a resolvable context — before a month name, before
  a lexicon noun (§ Ordinal noun lexicon), inside a numeric date, or after
  a trigger whose ending does not depend on the noun's gender (weak
  dative/accusative/genitive `am`/`vom`/`zum`/`beim`/`im`/`dem`/`den`/`des`
  → `-en`, weak nominative `die`/`das` → `-e`): `am 100. Geburtstag` →
  `am einhundertsten Geburtstag`, `Wir treffen uns am 5.` → `am fünften`.
  Elsewhere "N." is left alone; a wrong case ending is judged worse than
  the engine's own guess. Strong triggers (`seit`, `ab`, `bis`, `nach`,
  `vor`) and `der` still need a noun of known gender.
- **Ordinal noun lexicon (v1, fixed list, with gender)**: Klasse (f),
  Stock (m), Etage (f), Platz (m), Mal (n), Liga (f), Runde (f),
  Halbjahr (n), Quartal (n), Semester (n), Jahrhundert (n), Auflage (f),
  Weltkrieg (m), Preis (m), Sieg (m), Versuch (m), Kapitel (n), Teil (m),
  Geburtstag (m), Jahrestag (m), Jubiläum (n), Lebensjahr (n),
  Jahrtausend (n), Spieltag (m), Staffel (f), Folge (f), Ausgabe (f),
  Etappe (f), Generation (f), Woche (f), Monat (m), Jahr (n), Tag (m),
  Absatz (m). Not a dictionary — coverage grows only with a fixture that
  proves the context.
- **Source**: DIN 5008:2020; Duden — Die Grammatik §§ on ordinal
  declension; CLDR de `rbnf.json` (`spellout-ordinal`,
  `spellout-ordinal-n`, `spellout-ordinal-r`, `spellout-ordinal-s`,
  `spellout-ordinal-m`).

## Decimal

- **Written**: comma as the decimal separator (`3,5`, `0,75`) — DIN
  5008:2020 §5; CLDR de `numbers.json` decimal symbol.
- **Spoken**: integer part via `%spellout-numbering`, then "Komma", then
  each fraction digit read individually (Duden — Richtiges und gutes
  Deutsch, entry "Bruchzahlen"): `drei Komma fünf`, `null Komma sieben
  fünf`.
- **Source**: DIN 5008:2020; Duden — Richtiges und gutes Deutsch; CLDR de
  `rbnf.json` decimal (`x.x`) rule with the locale's `,` symbol.

## Dotted number

- **Written**: two or more digit groups joined by periods with no
  surrounding spaces (`2.3`, `3.10`, `1.2.7`, `Kapitel 2.3`, `Version
  1.01.5`) that no higher class claimed: not a Date (`1.11.2026`, `02.03`),
  not a grouped Cardinal (`1.000`, `1.000.000` — every group after the
  first has exactly three digits), not an Ordinal. Section, clause and
  version numbers as DIN 1421 numbers them.
- **Spoken**: each group as a Cardinal, joined by "Punkt" ("zwei Punkt
  drei", "drei Punkt zehn"); a group with a leading zero is read digit by
  digit ("eins Punkt null eins Punkt fünf"). Before this class the two
  numbers were read with the period left between them ("drei.zehn"),
  which a synthesiser takes for a sentence break.
- **Source**: DIN 1421 (section numbering); the "Punkt" reading is the
  usual spoken form of a version or section number (**choice**).

## Electronic

- **Written**: an `@` address (`max.mustermann@example.de`), a `scheme://`
  URL (`https://www.example.com/pfad?x=1`, `ftp://`, `file://`), `www.…`,
  a bare domain with a known TLD and optional path (`example.com`,
  `brettspielversand.de`), an IPv4 address (four dotted 1–3-digit groups)
  and an `@handle`. Bounded: not glued to a letter, digit, `@`, `.` or
  `/` on either side; a sentence-final `.`, `,`, `)` stays outside. The
  TLD list is fixed, so `z.B.`, `Dr.`, `usw.`, `3.10.` and `1.000` never
  match; `es` is dropped from the list for German (it is a word).
- **Spoken**: letter runs as written (the synthesiser spells or reads
  them), digits one by one, symbols by name:
  `@` "at", `.` "Punkt", `/` "Schrägstrich", `:` "Doppelpunkt", `?`
  "Fragezeichen", `=` "gleich", `&` "und", `-` "Bindestrich", `_`
  "Unterstrich", `#` "Raute", `+` "plus", `~` "Tilde", `%` "Prozent".
  `max.mustermann@example.de` → "max Punkt mustermann at example Punkt
  de"; `192.168.0.1` → "eins neun zwei Punkt eins sechs acht Punkt null
  Punkt eins"; `@jensen` → "at jensen". Highest priority of all classes.
- **Source**: design §7.2 "Electronic"; "at" for `@` is the usual German
  reading (Duden lists "At-Zeichen"), "Punkt"/"Schrägstrich"/"Doppelpunkt"
  are the DIN 5008 names of the signs (**choice**).

## Money

- **Written**: amount plus `€`/`EUR`/`Euro` before or after, or another
  ISO 4217 code/symbol (`8,80 €`, `€ 8,80`, `8,80 EUR`, `8,80 Euro`); a
  symbol glued before the amount (`€2,20`, `$2`) counts too.
- **Spoken**: one- or two-digit fraction is minor-unit cents
  (`acht Euro achtzig`); a bare `,00` fraction is dropped
  (`zehn Euro`, not `zehn Euro null`); an amount with only a fraction part
  reads the minor unit alone (`fünfzehn Cent`); three or more fraction
  digits fall back to the Decimal reading plus the currency name.
  `ein Euro`/`ein Cent` via the masculine cardinal agreement.
- **Agreement**: currency and minor-unit nouns are of known gender
  (masculine: Euro, Cent, Dollar, Franken; feminine minor units follow the
  ISO 4217 table), so `1` before them takes `ein`/`eine`.
- **Scale words**: `Tsd.`/`Tausend`, `Mio.`/`Mio`/`Million(en)`,
  `Mrd.`/`Milliarde(n)`, `Bio.`/`Billion(en)` between the amount and the
  currency join the Money span; the amount reads as a number (a comma is
  a decimal, never cents), the scale noun is feminine and plural from 2
  ("eine Million", "zwei Komma fünf Milliarden"), the currency noun is
  plural: `2,5 Mrd. €` → "zwei Komma fünf Milliarden Euro", `€ 3 Mio.` →
  "drei Millionen Euro". With no currency the number and scale are a
  `Scaled` span and the noun stays outside (`1,2 Mio. Einwohner` → "eins
  Komma zwei Millionen Einwohner"); a scale glued into a compound
  (`Millionenstadt`) is not one. (Duden: Million, Milliarde, Billion are
  nouns and take the plural; Tausend is invariant after a numeral.)
- **Source**: DIN 5008:2020 currency notation; CLDR de `numbers.json`
  (currency formatting patterns), `currencies.json` (display names), CLDR
  `supplemental/currencyData.json` (minor-unit digits); a small per-language
  minor-unit **name** table sourced from ISO 4217 conventions (Cent,
  Rappen, Pence, …) since CLDR does not name minor units.

## Percent, permille

- **Written**: `%`/`‰` directly or space-separated after a number (`12 %`,
  `12%`, `3,5 ‰`) — DIN 5008:2020 §5.
- **Spoken**: number via Cardinal/Decimal, then "Prozent"/"Promille".
- **Agreement**: "Prozent" and "Promille" are nouns of known (neuter)
  gender — CLDR de `units.json` `concentr-percent` and `concentr-permille`
  both carry `gender: "neuter"` — so a bare `1` before them takes `ein`
  (`1 %` → `ein Prozent`, `1 ‰` → `ein Promille`), same mechanism as
  Cardinal (§7.3). A decimal integer part before them stays bare "eins"
  regardless, since Decimal never applies noun agreement (`0,1 %` → `null
  Komma eins Prozent`).
- **Source**: DIN 5008:2020; Duden 28th ed. (symbol usage); CLDR de
  `numbers.json` percent/permille signs; CLDR de `units.json`
  (`concentr-percent`, `concentr-permille` gender).

## Degrees

- **Written**: `°`, `°C`, with an optional leading `-` (`20 °C`, `20°`,
  `-5 °C`).
- **Spoken**: number via Cardinal (with "minus" for a leading `-`), then
  "Grad", then the unit name if present ("Celsius").
- **Agreement**: "Grad" is a noun of known (neuter) gender — CLDR de
  `units.json` `angle-degree` carries `gender: "neuter"` — so a bare `1`
  before it takes `ein` (`1 °C` → `ein Grad Celsius`); this happens to
  read identically to the masculine nominative form since both take `ein`.
- **Source**: DIN 5008:2020; CLDR de `units.json` (`angle-degree` gender;
  temperature unit `celsius` long name).

## Measure

- **Written**: number plus a unit symbol or currency, optionally followed
  by `/` and a second unit or a time-period abbreviation (`10 km`,
  `80 m²`, `1,5 Liter`, `120 km/h`, `30 €/Monat`, `8 €/Std.`).
- **Spoken**: unit names from CLDR long forms with the correct plural
  category; symbols resolved through the short-form reverse index filtered
  by a per-language allow list (to keep ambiguous single letters like "m",
  "s", "h" out unless the context is unambiguous); `/` directly after a
  unit or currency becomes "pro"; `/` anywhere else is untouched
  (`Lehrer/Lehrerin` stays as written — it is not a measure).
- **Digital units**: CLDR `digital-*` (`Bit`, `Byte`, `kB`/`KB`, `MB`,
  `GB`, `TB`, `PB`, `kbit`, `Mbit`, `Gbit`, `Tbit`) read by their long
  names ("zwei Gigabyte", "ein Terabyte"); data rates as compounds
  (`100 Mbit/s` → "einhundert Megabit pro Sekunde") and the `bps`
  spellings (`Mbps`, `Gbps`, `kbps`) fold in the "pro Sekunde".
- **Source**: CLDR de `units.json` (long unit names, plural category,
  gender); a per-language symbol allow/deny list (not in CLDR); DIN
  5008:2020 for the `/`-as-"pro" convention.

## Time

- **Written**: `HH:MM` (colon form, always a time); `HH.MM Uhr` (dotted
  form, only a time when followed by "Uhr" — otherwise `3.10.` is a Date);
  `HH:MM:SS` with optional "Uhr".
- **Spoken**: hour and minute via Cardinal, joined by "Uhr"; `:00` collapses
  to "Uhr" alone (`neun Uhr`, not `neun Uhr null`); one o'clock is
  `ein Uhr` (Duden). The seconds form reads minutes and seconds as counted
  nouns: `02:02:23 Uhr` → `zwei Uhr zwei Minuten dreiundzwanzig Sekunden`,
  `1:01:01` → `ein Uhr eine Minute eine Sekunde`. A sports duration
  (`2:02:23` as hours:minutes:seconds elapsed) is not distinguished from a
  clock reading — open.
- **Time zones**: a zone name after the time (`14:30 CET`, `14:30 Uhr
  MEZ`, `09:00 UTC`) is outside the span and stays as written — a
  phonemiser spells the capitals itself, and no zone list is maintained.
  A `UTC`/`GMT` offset reads as an expression: `12:00 UTC+2` → "zwölf Uhr
  UTC plus zwei".
- **Source**: DIN 5008:2020 (time notation); CLDR de `rbnf.json`
  `spellout-numbering` for hour/minute.

## Time range

- **Written**: two Time-shaped values joined by an unspaced en dash or the
  word "bis" (`14:00–16:00`, `19.30–21.00 Uhr`, `von 7:00 bis 9:00 Uhr`,
  `10-14 Uhr`).
- **Spoken**: both ends read as Time, joined by "bis"; higher classifier
  priority than plain Time so both ends of a range are read alike (a lone
  Time recogniser could otherwise strip "Uhr" from only one end).
- **Source**: DIN 5008:2020 (Bis-Strich, §12.3).

## Date

- **Written**: `D.M.YYYY`/`DD.MM.YYYY` (DIN numeric date), `D.M.` with
  the trailing period, zero-padded `DD.MM` without it (`02.03`; unpadded
  `3.10` is a section or version number, see Dotted number), ISO
  `YYYY-MM-DD`, ISO year-month `YYYY-MM` (`2003-03`, only with a
  zero-padded month 01–12 and not continuing into another `-` digit
  group), or `D. Monatsname [YYYY]`.
- **Spoken**: day as an Ordinal with agreement, month from the CLDR wide
  name, year via the Year rule when present. The ISO form is read the
  German way (day, then month, then year), not literally left-to-right;
  the year-month form is month then year ("März zweitausenddrei").
- **Source**: DIN 5008:2020 §5.5 (date notation); ISO 8601; CLDR de
  `ca-gregorian.json` (wide month names); Ordinal and Year rules above.

## Range

- **Written**: two values joined by the unspaced Bis-Strich (en dash or
  plain hyphen with no surrounding spaces) — `5–10 Minuten`, `2010–2012`,
  `5-10`, `1.–3. November`. A **spaced** " – " is a Gedankenstrich (an
  em/en dash used as a sentence-level punctuation mark) and is left alone,
  since DIN 5008 reserves the unspaced form for ranges.
- **Spoken**: both ends read per their own class (Cardinal, Year, Ordinal),
  joined by "bis". A Date range applies the same Ordinal agreement to both
  ends (`vom ersten bis dritten November`).
- **Source**: DIN 5008:2020 §12.3 (Bis-Strich vs. Gedankenstrich).

## Score

- **Written**: `N:M`.
- **Spoken**: `N zu M` via Cardinal on both sides.
- **Priority**: lower than Time, so only an `N:M` pair that is not a valid
  clock time (hour ≤ 23, minute ≤ 59) is read as a score.
- **Source**: Duden — Richtiges und gutes Deutsch (sports-result notation).

## Telephone

- **Written**: requires an explicit phone shape per DIN 5008:2020 §12.4 —
  a `+` country prefix, or a leading-zero group (optionally in parentheses,
  `(0411) 1234-1234`) followed by `/`, `-`, or a space and at least one
  more digit group, with at least seven digits in total. `.` and `:` are never phone separators (so `01.11.2026` and
  `09:00` cannot match).
- **Spoken**: digit by digit; groups (as written) separated by a pause
  comma.
- **Source**: DIN 5008:2020 §12.4 (Telefonnummern).

## Long digit run

- **Written**: any digit run with a leading zero that no higher-priority
  class claimed (IBANs, reference numbers: `007`), or any run whose value
  overflows the `i128` number type.
- **Spoken**: digit by digit; `Span.fallback = true`, so the survey tool
  can report it as a shape the higher-priority classes should learn.
- **Source**: design §7.2 "Long digit run" row (no external written-form
  standard governs arbitrary reference-number strings).
- **Digit groups** (`Token::DigitGroups`): a 16-digit card number in four
  groups (`4111 1111 1111 1111`, hyphens or spaces) reads digit by digit
  with a pause comma between groups, like Telephone, `fallback = false`.
  Placed below IBAN and above Telephone.

## Abbreviation

- **Written**: the fixed Duden abbreviation list (§ below), with or
  without inter-word spaces (`z. B.`, `z.B.`).
- **Spoken**: the expansion from the list, only when
  `Options::expand_abbreviations` is true (default). Weekdays `Mo.`–`So.`
  and months `Jan.`–`Dez.` expand via the CLDR de abbreviated names
  instead of a hand table. A trailing unit-like shorthand after a cardinal
  (`12 J.` → "zwölf Jahren") is a listing convention, not a general
  abbreviation.
- **Source**: Duden — Die deutsche Rechtschreibung (28th ed.), abbreviation
  appendix; Wiktionary "Kategorie:Abkürzung (Deutsch)" cross-checked
  against Duden (see Attribution below); CLDR de `ca-gregorian.json`
  (abbreviated weekday/month names).

## Unicode fractions

- **Written**: `½`, `¼`, `¾` (precomposed Unicode fraction characters
  only).
- **Spoken**: `ein halb`, `ein Viertel`, `drei Viertel`.
- **Also classified**: ASCII slash fractions (`1/2`) under a
  numerator/denominator gate — see "Slash fraction" below.
- **Source**: design §7.2 "Unicode fractions" row.

## Roman numeral (clinical grading)

- **Written**: `I`, `II`, `III`, `IV` only, word-bounded, in clinical
  grading contexts (`NYHA II-III`, `Stadium IV`, `Grad III`, `Typ II`,
  `Mallampati III`, `Billroth II`, `Klasse I C`).
- **Spoken**: the German cardinal (`eins`, `zwei`, `drei`, `vier`). A
  hyphen between two such numerals is a range ("bis"); a hyphen with a
  non-numeral word on either side is a German compound and only the
  numeral is replaced (`Klasse-I-Empfehlung` → `Klasse-eins-Empfehlung`).
  `II`–`IV` stand on their own; a lone `I` is the numeral only directly
  after a grading word (Klasse, Grad, Typ, Stadium, NYHA, Mallampati,
  Billroth, Ableitung, Stufe, Phase, Kategorie, Gruppe, Weltkrieg, Band,
  Teil, Kapitel, Akt, Abschnitt, Generation), because `I` also occurs as
  the element symbol left in a spelled chemical formula (`I₂`) and in
  spelled output that a second pass must not re-read (§9 idempotence).
  A numeral glued to a sub- or superscript is never expanded.
- **Out of scope**: `V` and above are never expanded. A bare `V` is far
  more often "Vena" (`V. cava inferior`) or the Sozialgesetzbuch (`SGB V`)
  than the numeral five, and nothing in the written form disambiguates the
  two without a parser, so the design deliberately stops at `IV`.
- **Source**: design §7.2 "Roman numeral" row; German clinical grading
  conventions (NYHA, Mallampati, Billroth staging, Sozialgesetzbuch
  citation practice).

## Paragraph sign

- **Written**: `§`, `§§`, with or without a following space, before a
  number (`§ 25`, `§§ 25, 26`, `§25`).
- **Spoken**: `§` → "Paragraf", `§§` → "Paragrafen". A literal symbol
  substitution, like Abbreviation — it does not parse or consume the
  following number, which reads via the ordinary Cardinal rule.
- **Source**: design §7.6 "Paragraph sign" row; DIN 5008:2020 (legal
  citation abbreviation practice).

## Scientific notation

- **Written**: mantissa times ten to a signed exponent, in any of the
  notations a German science/medical text uses: `1,5 × 10⁻⁶`,
  `1,5 · 10⁻⁶`, `1,5×10^-6`, `1,5e-6`, `1.5E+08`; optionally followed by a
  unit (`6,022 · 10²³ mol⁻¹`).
- **Spoken**: mantissa via Decimal, "mal zehn hoch", exponent via Cardinal
  (with "minus" for a negative exponent): `eins Komma fünf mal zehn hoch
  minus sechs`; with a unit, `sechs Komma null zwei zwei mal zehn hoch
  dreiundzwanzig pro Mol`.
- **Gate**: the ASCII `e`/`E` form is classified only when the mantissa
  has a decimal separator or the exponent is explicitly signed, so a bare
  `1e10`, hex literals (`0x1F`), and alphanumeric IDs never match.
- **Priority**: above Decimal and Measure — the class owns the whole
  mantissa/exponent/unit span so those classes never see its digits.
- **Source**: DIN 1338 (Formelschreibweise); design §7.6 "Scientific
  notation" row.

## Power / exponent

- **Written**: Unicode superscript digits (U+2070 block) or ASCII `^n`
  directly after a bare number or a single Latin/Greek-letter variable:
  `2⁵`, `10³`, `x²`, `2^10`, `x^-2`.
- **Spoken**: always "hoch n" (`zwei hoch fünf`, `x hoch zwei`, `x hoch
  minus zwei`) — "Quadrat"/"Kubik" are never produced by this class; they
  are already CLDR's own compound unit names (`m²` → "Quadratmeter") and
  Measure claims those tokens first, at higher priority.
- **Note**: charge notation (`²⁺`, `⁻`) after an element symbol is
  Chemical formula, not this class.
- **Source**: DIN 1338; design §7.6 "Power / exponent" row.

## Chemical formula

- **Written**: Unicode subscript digits (U+2080 block) glued to a letter
  (`H₂O`, `CO₂`, `C₆H₁₂O₆`), and superscript charge notation after an
  element symbol (`Ca²⁺`, `Cl⁻`).
- **Spoken**: only the subscript digits are replaced by the cardinal; the
  letters stay as written and are spelled by the engine's own
  glued-letter-and-digit reading (§9 of the design): `H zwei O`, `CO zwei`,
  `C sechs H zwölf O sechs`. A trailing charge superscript reads
  "plus"/"minus" after the digit: `Ca zwei plus`, `Cl minus`.
- **Not classified (needs no rule)**: plain ASCII `H2O`/`CO2` already read
  correctly through the existing glued-digit-and-letter cardinal rule
  (§9) — "H zwei O", "CO zwei" — with no dedicated rule required.
- **Source**: design §7.6 "Chemical formula" row.

## Math expression

- **Written**: a recognised operator between a numeric operand and a
  numeric or single-letter operand, spaces around the operator optional:
  `3 + 4 = 7`, `p < 0,05`, `n = 12`, `±2 %`, `√2`, `x ≈ 3,14`, `≥65 Jahre`,
  `> 37 pmol/l`.
- **Spoken**: `+` plus, unspaced-both-sides `-`/`−` minus, `×`/`·`/`*`
  mal, spaced-both-sides `÷`/`:` geteilt durch, `=` gleich, `≠` ungleich,
  `<` kleiner als, `>` größer als, `≤` kleiner oder gleich, `≥` größer
  oder gleich, `≈` ungefähr, `±` plus minus, `√` Wurzel aus, `∞`
  unendlich, `π` Pi.
- **Gate**: a numeric operand on at least one side and a numeric or
  single-letter (Latin/Greek) operand on the other — `a = b` alone (no
  digit) is left untouched. The four comparison operators (`<`, `>`,
  `≤`, `≥`) additionally allow a right-hand-only operand (prefix use, as
  clinical findings are actually written). An unspaced `-` between two
  numbers stays Range (§7.2), not this class.
- **Priority**: above Decimal, Percent and Measure, for the same reason
  as Scientific notation — a trailing unit or `%` is read inside the same
  span.
- **Source**: DIN 1338 (relation and operator symbols); German clinical
  documentation conventions for comparison-operator wording (Rote Liste,
  Fachinformationen, Laborbefunde); design §7.6 "Math expression" row.

## Slash fraction

- **Written**: `1/2`, `3/4`, `2/3`, `1/8`; mixed `1 ½`.
- **Spoken**: `ein halb`, `drei Viertel`, `zwei Drittel`, `ein Achtel`,
  `ein einhalb`.
- **Gate**: a bare `N/M unit` (`1/2 kg`) is a fraction, not a rate — a
  Measure per-segment carries a number only after a numerator unit
  (`ml/min/1,73 m²`); the unit symbol after a fraction stays as written
  (`ein halb kg`) — open.
  Numerator < denominator; denominator ≤ 99, or ∈ {100, 1000} — 2–12 and
  100/1000 from the hand table, 13–99 derived from the cardinal with
  "-tel" (13–19: "Dreizehntel") or "-stel" (20–99: "Zwanzigstel",
  "Zweiundzwanzigstel"), Duden;
  not adjacent to another digit-slash group or a date-like shape
  (`1/2/2026`); not preceded by a unit or currency symbol (that `/` stays
  "pro" via Measure, which outranks this class).
- **Denominator table** (fixed, ordinal stem + "-el", irregular `halb`):
  2 halb, 3 Drittel, 4 Viertel, 5 Fünftel, 6 Sechstel, 7 Siebtel,
  8 Achtel, 9 Neuntel, 10 Zehntel, 11 Elftel, 12 Zwölftel, 100 Hundertstel,
  1000 Tausendstel.
- **Known limitation**: adjective agreement before a noun is not
  implemented — `½ Liter`/`1/2 Liter` reads "ein halb Liter", not the
  grammatically correct "ein halber Liter" (design §7.2 "Out of scope",
  §14).
- **Source**: Duden — Richtiges und gutes Deutsch (Bruchzahlen); design
  §7.6 "Slash fraction" row.

## Ratio, titer and blood pressure

- **Written**: `120/80 mmHg`, `RR 120/80`, `100-110/min`,
  `Verhältnis 1:3`, `1 : 3`, `ANA 1:640`, `Titer 1:80`, `1:25.000`.
- **Spoken**: `hundertzwanzig zu achtzig Millimeter Quecksilbersäule`,
  `RR hundertzwanzig zu achtzig`, `hundert bis hundertzehn pro Minute`,
  `eins zu drei`, `eins zu drei`, `eins zu sechshundertvierzig`,
  `eins zu achtzig`, `eins zu fünfundzwanzigtausend`.
- **Gate**: `N/M` reads "zu" (not "pro") when followed by `mmHg` or
  preceded by the bare token `RR` (`RR` itself is left as written — a
  caller wanting "Blutdruck" supplies it via `Options::lexicon`, design
  §5). A `N-M/UNIT` range immediately followed by a rate unit reads both
  ends via Range wording and the unit once. `N:M` reads "zu" when preceded
  by a trigger word (Titer, Verhältnis, Mischung, Maßstab, Chance, Quote,
  ANA, Verdünnung) or when `M` is not a plausible clock minute (`M > 59`
  or 3+ digits) — the same wording Score already produces (§7.2); a bare
  `2:1` with no trigger and a plausible-score shape still falls to Score.
- **Priority**: above Measure (so `mmHg`/rate units are claimed whole)
  and above Range and Score.
- **Source**: DIN 5008:2020 (ratio/measurement notation); CLDR de
  `units.json` (`pressure-millimeter-ofhg`); German laboratory and
  vital-sign documentation conventions (blood-pressure/titer notation).

## Dose scheme

- **Written**: `20-5-0 mg/d`, `1-0-1`, `1-1-1-1`, `0-0-1`.
- **Spoken**: `zwanzig, fünf, null Milligramm pro Tag`, `eins, null,
  eins`, `eins, eins, eins, eins`, `null, null, eins`.
- **Gate**: exactly three or four groups of 0–99 (or `½`) joined by an
  unspaced hyphen, where the largest group is ≤ 200 or the token sits
  next to a dosing-unit/context word (mg, ml, Tbl., Tablette(n),
  Kapsel(n), IE, Tropfen).
- **Spoken rule**: groups read in order via Cardinal, separated by the
  Telephone convention's pause comma (design §7.2) — never "minus"; not
  collapsed into a Range even though it is hyphen-digit-hyphen-digit
  shaped like one. A trailing unit reads once, after the last group.
- **Priority**: above Range and above Measure, so the whole scheme is one
  span.
- **Source**: Rote Liste/Fachinformationen package-insert dose-scheme
  convention (morning-midday-evening, or morning-midday-evening-night).

## Repetition / multiplication

- **Written**: `2x täglich`, `2×/Tag`, `4x 500 mg`, `3-mal`, `1× Insulin`,
  `1,5 x 2 cm`.
- **Spoken**: `zweimal täglich`, `zweimal pro Tag`, `viermal fünfhundert
  Milligramm`, `dreimal`, `einmal Insulin`, `eins Komma fünf mal zwei
  Zentimeter`.
- **Gate**: `N` (`x`|`×`) fuses into one cardinal+"mal" word (`einmal`,
  `zweimal`, `dreimal`, …) whenever `N` is a plain integer, regardless of
  what follows — a word, a rate, a further number, or nothing. It stays a
  separate "mal" only when `N` itself is a decimal (`1,5 x 2 cm`): a
  decimal cardinal phrase does not idiomatically take the "-mal" suffix.
  Already-spelled `N-mal` reads the same fused way (no-op).
- **Priority**: below Measure, above Range.
- **Source**: German dosing-frequency documentation conventions (Rote
  Liste, Fachinformationen); design §7.6 "Repetition / multiplication" row.

## Angle / coordinate

- **Written**: `30°15′`, `30° 15′ 20″`, `52,52° N`.
- **Spoken**: `dreißig Grad fünfzehn Minuten`, `dreißig Grad fünfzehn
  Minuten zwanzig Sekunden`, `zweiundfünfzig Komma fünf zwei Grad Nord`.
- **Rule**: prime `′`/ASCII `'` directly after a degree value reads
  "Minuten"; double prime `″`/ASCII `"` reads "Sekunden" — the bare
  sexagesimal-chain reading the ISO 6709/surveying convention uses, not
  CLDR's full `angle-arc-minute`/`-second` unit names (a deliberate
  deviation: the "Grad … Minuten … Sekunden" chain already disambiguates
  without the fuller "Winkelminute" wording — design §14). A single
  compass letter (`N`, `O`, `S`, `W`) directly after a degree/prime/second
  value expands to the cardinal-direction word (Nord, Ost, Süd, West).
- **Priority**: above Degrees, which alone only recognises a bare
  `°`/`°C` group.
- **Source**: ISO 6709 (geographic coordinate notation); design §7.6
  "Angle / coordinate" row.

## Measure — scientific and medical unit extension

- **Written**: `mmHg`, `mmol/l`, `mol`, `mmol`, `Ω`, `kΩ`, `Hz`, `kHz`,
  `MHz`, `GHz`, `K` (only after a digit and a space, e.g. `273 K`), `V`,
  `mV`, `A`, `mA`, `W`, `kW`, `MW`, `Pa`, `hPa`, `kPa`, `bar`, `mbar`,
  `Bq`, `Gy`, `Sv`, `lx`, `lm`, `cd`, `N` (unit context), `J`, `kJ`, `eV`,
  `keV`, `MeV`, `ly`, `Lj`, `pc`, `AE`, `AU`, `ppm`, `kcal`; plus compound
  lab tokens `ml/min/1,73 m²`, `mg/kgKG`, `kg/m²`, `mmol/mol`,
  `mosmol/kg`, `µmol/l`, `nmol/l`, `pmol/l`, `mval/l`, `mEq/l`, `mg/kg`,
  `mg/dl`, `mg/l`, `mg/g`, `µg/dl`, `µg/l`, `ng/ml`, `ng/l`, `pg/ml`,
  `g/dl`, `g/l`, `Gpt/l`, `G/l`, `IU/ml`, `IU/l`, `kU/l`, `mU/l`, `U/l`,
  `mm/h`, `/µl`, `/nl`, `/min`, `/d`, `/Tag`, `/h`, `500mg` (no space),
  `75-g-oGTT`, `10-g-Monofilament`.
- **Spoken, via CLDR de `units.json`** (id in brackets): Millimeter
  Quecksilbersäule (`pressure-millimeter-ofhg`), Millimol pro Liter
  (`concentr-millimole-per-liter`), Mol/Millimol (`concentr-mole`),
  Ohm (`electric-ohm`), Hertz/Kilohertz/Megahertz/Gigahertz
  (`frequency-hertz`/`-kilohertz`/`-megahertz`/`-gigahertz`), Kelvin
  (`temperature-kelvin`), Volt/Millivolt (`electric-volt`),
  Ampere/Milliampere (`electric-ampere`/`-milliampere`),
  Watt/Kilowatt/Megawatt (`power-watt`/`-kilowatt`/`-megawatt`),
  Pascal/Hektopascal/Kilopascal/Bar/Millibar
  (`pressure-pascal`/`-hectopascal`/`-kilopascal`/`-bar`/`-millibar`),
  Becquerel (`energy-becquerel`), Gray (`energy-gray`), Sievert
  (`energy-sievert`), Lux/Lumen/Candela (`light-lux`/`-lumen`/`-candela`),
  Newton (`force-newton`), Joule/Kilojoule (`energy-joule`/`-kilojoule`),
  Elektronenvolt (`energy-electronvolt`, kilo/mega composed the way `kJ`
  already is), Lichtjahr (`length-light-year`), Parsec
  (`length-parsec`), Astronomische Einheit (`length-astronomical-unit`),
  Millionstel (`concentr-part-per-1e6` — confirmed present in
  `xtask/cldr/cldr-units-full/main/de/units.json`, so `ppm` translates
  instead of being spelled letter-by-letter), Kilokalorien
  (`energy-kilocalorie`).
- **Extra-symbol table** (no CLDR de unit exists — confirmed absent from
  `xtask/cldr/cldr-units-full/main/de/units.json` under every key checked:
  `force-kilonewton`, any `*decibel*`, any `*millisievert*`,
  `concentr-part-per-million` under that literal name): `IE`/`I.E.`/`IU`
  → Internationale Einheiten, `kU` → Kiloeinheiten, `mU` →
  Millieinheiten, `U` → Einheiten, `Gpt` → Gigapartikel, `Tpt` →
  Terapartikel, `G` (only the bare symbol, before `/l` only,
  case-sensitive) → Giga, `mval` → Millival, `mEq` →
  Milliäquivalent, `mosmol` → Milliosmol, `KG` (only inside the welded
  `mg/kgKG`) → Kilogramm Körpergewicht, `sek` → Sekunden, `kN` →
  Kilonewton, `mSv` → Millisievert, `dB` → Dezibel.
- **Compositional compound reading**: `A/B` and `A/B/C` tokens are
  resolved segment by segment through the CLDR short-symbol reverse index
  or the extra-symbol table above, joined by "pro" — not one regex per
  compound. Longer compounds are tried before their own prefix
  (`ml/min/1,73 m²` before plain `ml`), extending the "specific before
  general" ordering Measure already uses for `km/h` vs. `km`.
- **Case sensitivity**: `G/l` ("Giga pro Liter") and `g/l` ("Gramm pro
  Liter") are different lab values and are never case-folded together.
- **`pH 7,4`**: needs no new rule — `pH` is untouched text and `7,4`
  already reads via Decimal ("sieben Komma vier"); listed here only as a
  shape confirmation.
- **Source**: CLDR de `units.json` (ids above); German laboratory
  documentation conventions for the compound-unit shapes.

## Clinical shorthand lexicon

- `SpO2`→"Sauerstoffsättigung", `RR`→"Blutdruck", `HF`→"Herzfrequenz",
  `AF`→"Atemfrequenz", `BMI`→"B M I", `GFR`→"G F R" are application-domain
  vocabulary, not German-language rules — they are not built into the
  classifier. A caller supplies them through `Options::lexicon` (design
  §5): a literal, case-sensitive, word-bounded substitution list applied
  after every digit-bearing class, so it can only replace letters the
  digit classes left untouched and can never shadow a phone number, dose,
  or ratio span.
- **Source**: design §5, §7.6; German clinical-shorthand conventions.

## §7.3 Agreement (German)

Agreement is decided by the classifier from local surface context — there
is no parser — and is fixed to exactly these rows:

| Context | Agreement | Ordinal ending | RBNF ruleset |
|---|---|---|---|
| `am / vom / zum / beim / im` + N. | weak dative | `-en` | `%spellout-ordinal-n` |
| `den / dem / des` + N. | weak | `-en` | `%spellout-ordinal-n` |
| `der` + N. + masculine noun | weak nominative | `-e` | `%spellout-ordinal` |
| `der` + N. + feminine noun | weak dative/genitive | `-en` | `%spellout-ordinal-n` |
| `die / das` + N. | weak nominative | `-e` | `%spellout-ordinal` |
| `seit / ab / bis / nach / vor` + N. + month | strong dative | `-em` | `%spellout-ordinal-m` |
| bare N. + month (no trigger) | strong nominative | `-er` | `%spellout-ordinal-r` |
| bare N. + noun | strong nominative by gender | `-er`/`-e`/`-es` | `%spellout-ordinal-r`/`%spellout-ordinal`/`%spellout-ordinal-s` |

Trigger matching is case-insensitive at sentence start. The same mechanism
selects "ein/eine/eins" for cardinals: a following noun of known gender
(from the unit, currency, or ordinal noun lexicon tables) selects
`%spellout-cardinal-masculine`, `-feminine`, or `-neuter`; otherwise the
bare `%spellout-numbering` reading ("eins") is used.

**Source**: Duden — Die Grammatik, ordinal and weak/strong adjective
declension tables; CLDR de `rbnf.json` ordinal rulesets
(`spellout-ordinal`, `-n`, `-r`, `-s`, `-m`).

## Abbreviation list (~100 entries)

Curated from Wiktionary's "Kategorie:Abkürzung (Deutsch)", cross-checked
against Duden, to the set a TTS context plausibly needs — common
administrative, academic, and everyday abbreviations, plus DIN/Duden
abbreviations named explicitly in design §7.2. Weekday and month
abbreviations are generated from CLDR, not from this table, and are listed
separately below for completeness.

| Abbreviation | Expansion |
|---|---|
| Abs. | Absatz |
| Abt. | Abteilung |
| allg. | allgemein |
| bzgl. | bezüglich |
| bzw. | beziehungsweise |
| ca. | zirka |
| d. h. | das heißt |
| Dipl. | Diplom |
| Dr. | Doktor |
| evtl. | eventuell |
| Fa. | Firma |
| Fr. | Frau (before a name) / Freitag (weekday context) — ambiguous, resolved by context |
| geb. | geboren |
| gegr. | gegründet |
| ggf. | gegebenenfalls |
| Hr. | Herr |
| i. d. R. | in der Regel |
| inkl. | inklusive |
| insb. | insbesondere |
| Jh. | Jahrhundert |
| Kap. | Kapitel |
| m. E. | meines Erachtens |
| Mio. | Million |
| Min. | Minute |
| Mrd. | Milliarde |
| MwSt. | Mehrwertsteuer |
| Nr. | Nummer |
| o. Ä. | oder Ähnliches |
| Pf. | Pfennig |
| Prof. | Professor |
| S. | Seite |
| s. o. | siehe oben |
| s. u. | siehe unten |
| Sek. | Sekunde |
| St. | Stück |
| Std. | Stunde |
| Str. | Straße |
| Tel. | Telefon |
| u. a. | unter anderem |
| u. Ä. | und Ähnliches |
| u. U. | unter Umständen |
| usw. | und so weiter |
| v. a. | vor allem |
| v. Chr. | vor Christus |
| n. Chr. | nach Christus |
| vgl. | vergleiche |
| z. B. / z.B. | zum Beispiel |
| z. T. | zum Teil |
| zzgl. | zuzüglich |
| Bhf. | Bahnhof |
| Hbf. | Hauptbahnhof |
| etc. | et cetera |
| Abk. | Abkürzung |
| Anm. | Anmerkung |
| Aufl. | Auflage |
| bspw. | beispielsweise |
| dgl. | dergleichen |
| ebd. | ebenda |
| entspr. | entsprechend |
| excl. | exklusive |
| ff. | fortfolgende |
| Hrsg. | Herausgeber |
| i. A. | im Auftrag |
| Kfm. | Kaufmann |
| Kto. | Konto |
| led. | ledig |
| lfd. | laufend |
| o. g. | oben genannt |
| o. J. | ohne Jahr |
| Pkt. | Punkt |
| resp. | respektive |
| sog. | sogenannte — the abbreviation stands for the attributive adjective, whose most frequent surface form is the weak/plural "sogenannte" (`eine sog. Pseudothrombophlebitis`); full agreement would need the noun's gender and case |
| Tsd. | Tausend |
| u. dgl. | und dergleichen |
| u. v. a. | unter vielen anderen |
| verh. | verheiratet |
| Vers. | Versicherung |
| Verf. | Verfasser |
| Wdh. | Wiederholung |
| z. Zt. | zur Zeit |
| zit. | zitiert |
| zzt. | zurzeit |

### Medical abbreviations

Matching is case-sensitive: `V. a.` and `v. a.` share every letter and
differ only in capitalisation, but mean different things.

| Abbreviation | Expansion |
|---|---|
| V. a. | Verdacht auf |
| v. a. | vor allem |
| Z. n. | Zustand nach |
| i. v. | intravenös |
| i. m. | intramuskulär |
| s. c. | subkutan |
| p. o. | per os |
| I. E. | Internationale Einheiten |
| sog. | sogenannte |
| bspw. | beispielsweise |
| mind. | mindestens |
| max. | maximal |
| bds. | beidseits |
| vs. | versus |
| Abb. | Abbildung |
| insb. | insbesondere |

Deliberately excluded: the bare Latin single-letter anatomical
abbreviations `M.`/`A.`/`V.` (Musculus/Arteria/Vena). A lone letter is
ambiguous on its own (`M.` is also the abbreviation for Morbus) without
the noun that follows to disambiguate, and the written form never records
which was meant.

Weekdays (CLDR de `ca-gregorian.json` abbreviated names): Mo. → Montag,
Di. → Dienstag, Mi. → Mittwoch, Do. → Donnerstag, Fr. → Freitag,
Sa. → Samstag, So. → Sonntag.

Months (CLDR de `ca-gregorian.json` abbreviated names, periods only where
CLDR uses one): Jan. → Januar, Feb. → Februar, März → März, Apr. → April,
Mai → Mai, Juni → Juni, Juli → Juli, Aug. → August, Sep. → September,
Okt. → Oktober, Nov. → November, Dez. → Dezember.

## Attribution

- **Wiktionary** — the abbreviation → expansion fact table above draws its
  entries from the German Wiktionary category "Kategorie:Abkürzung
  (Deutsch)" (https://de.wiktionary.org/wiki/Kategorie:Abk%C3%BCrzung_(Deutsch)),
  licensed CC BY-SA 4.0. Only the curated *list* of abbreviations and their
  standard expansions is reused as a fact table, cross-checked against
  Duden; no Wiktionary prose is copied.
- **NeMo test inputs** — German fixture input shapes are cross-checked
  against `tests/nemo_text_processing/de/data_text_normalization/*.txt`
  from NVIDIA NeMo (`nemo-text-processing`), licensed Apache-2.0. Only
  input shapes are imported; every expected output in this repository's
  fixtures is re-derived by hand against DIN 5008:2020 and Duden, since
  NeMo's own expected outputs disagree with this design in places (e.g.
  NeMo reads "1 €" as "eins million euro"-style constructions in some
  paths, and only recognises the symbol-before-amount money form).
