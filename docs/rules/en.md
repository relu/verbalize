# English (`en`) — rule provenance

One section per semiotic class, in the order of the German rule doc,
for the English module. Where neither CLDR nor a style guide fixes an
English reading, the choice made here is marked **choice** and is the
open question for review; every such reading has a fixture in
`verbalize/tests/fixtures/en/`.

Written-form sources: The Chicago Manual of Style (17th ed.) for numerals,
dates and times; ISO 8601 for the ISO date; NANP (North American
Numbering Plan) and E.123 for telephone shapes; CLDR en. Spoken-form
sources: CLDR en RBNF (`%spellout-numbering`, `%spellout-numbering-year`,
`%spellout-ordinal`), CLDR en `units.json` and `currencies.json`.

Regional variants (`Options::region`): `EnUs` is the default; `EnGb`
changes numeric and ISO date order (§Date). Nothing else differs in v1.

## Cardinal

- **Written**: comma or space grouping (`1,000`, `22 000`); `-` sign.
- **Spoken**: CLDR en RBNF `%spellout-numbering` ("one thousand",
  "twenty-two thousand"). No gender agreement; "1 dollar"/"2 dollars"
  come from the CLDR plural category of the following noun's own class
  (Money, Measure).
- **Source**: Chicago 9.54 (grouping); CLDR en `rbnf.json`.

## Year

- **Written**: bare four-digit number 1000–2999 (`1066`, `1990`, `2024`);
  decade with `s` (`1990s`, `the 2000s`).
- **Spoken**: `%spellout-numbering-year` ("ten sixty-six", "nineteen
  ninety", "twenty twenty-four", "two thousand five"). Decades: the year
  reading plus "-s", with "-y" → "-ies" ("nineteen nineties", "nineteen
  hundreds", "two thousands"). A grouped `1,990` is a Cardinal.
- **Choice**: English has no lower bound like German's 1100; `1000` reads
  "one thousand" either way, and from 1010 the year ruleset takes over.
- **Source**: CLDR en `rbnf.json` (`%spellout-numbering-year`, `%%2d-year`).

## Ordinal

- **Written**: digits plus suffix (`1st`, `2nd`, `3rd`, `4th`, `22nd`),
  word-bounded. Unlike German, no context is needed: the suffix is the
  marker, so `1st` reads everywhere and a bare `5.` never does.
- **Spoken**: `%spellout-ordinal` ("first", "twenty-second").
- **Priority**: above Money and Measure, because `st`, `nd` and `rd` are
  also CLDR unit symbols (stone, nano-day, rod) that Measure would
  otherwise resolve.
- **Source**: Chicago 9.6; CLDR en `rbnf.json`.

## Decimal

- **Written**: point as the separator (`3.5`, `0.75`, `1,000.50`).
- **Spoken**: `x.x` rule of `%spellout-numbering` — "three point five";
  fraction digits one by one, trailing zeros included ("one thousand point
  five zero").
- **Source**: CLDR en `numbers.json` decimal symbol; RBNF `x.x` rule.

## Dotted number

- **Written**: three or more digit groups joined by periods (`1.2.7`,
  `4.02.1`); with one period the shape is a Decimal (`2.3`, `3.10`), and
  a dotted phone shape (`192.168.0.1`) is claimed by Telephone first.
- **Spoken**: each group as a Cardinal joined by "point" ("one point two
  point seven"); a leading-zero group digit by digit ("four point zero
  two point one"). Previously "one point two" was followed by a bare
  period and the next number.
- **Source**: usual spoken form of version and clause numbers (**choice**).

## Electronic

- **Written**: as in German — `@` addresses, `scheme://` URLs, `www.…`,
  bare domains with a known TLD (`example.com`, `www.ourdailynews.com/123-sm`),
  IPv4, `@handles`; `at`, `it`, `me`, `co` are dropped from the TLD list
  for English (they are words), so `e.g.`, `Dr.`, `5.4` never match.
- **Spoken**: letters as written, digits one by one, `@` "at", `.` "dot",
  `/` "slash", `:` "colon", `?` "question mark", `=` "equals", `&` "and",
  `-` "dash", `_` "underscore", `#` "hash", `+` "plus", `~` "tilde", `%`
  "percent": `a.bc@gmail.com` → "a dot bc at gmail dot com",
  `https://www.example.com/path?x=1` → "https colon slash slash www dot
  example dot com slash path question mark x equals one". Scheme letters
  and `www` stay lower-case as written (**choice**).

## Money

- **Written**: symbol before the amount, glued or with a space (`$8.80`,
  `US$ 100`, `£5`, `€0.99`, `$.01` → "one cent"); ISO code or currency word after (`8.80 USD`,
  `1 dollar`, `2 dollars`, `10 CHF`); a `/` rate after the currency
  (`$5/month`, `$30/hr`).
- **Spoken**: head noun of the CLDR display name by plural category
  ("US dollar" → "dollar"/"dollars", "British pounds" → "pounds",
  "euro"/"euros"); two or one fraction digits are the minor unit joined
  with "and" ("eight dollars and eighty cents", "twelve dollars and fifty
  cents", "one dollar and one cent"); minor unit alone below one ("fifteen
  cents", "fifty pence"); `.00` dropped; three or more fraction digits read
  as a decimal plus the currency ("eight point eight zero eight dollars").
  Minor units are a hand table: cent/cents, penny/pence, centime/centimes.
- **Scale words**: `thousand`, `million`, `billion`/`bn`, `trillion`, and
  the glued letters `k`/`M` (`$5k`, `$5M`; `$5 M` is not a scale) between
  amount and currency: `$2.5 billion` → "two point five billion dollars",
  `3 million euros` → "three million euros". No currency → a `Scaled`
  span ("five thousand" in `5 thousand people`).
- **Source**: CLDR en `currencies.json`; ISO 4217 minor units.

## Percent, permille

- **Written**: `12 %`, `12%`, `3.5 ‰`.
- **Spoken**: CLDR en `concentr-percent`/`concentr-permille` ("percent",
  "permille").

## Degrees

- **Written**: `68 °F`, `20 °C`, `20°`, `-5 °C`.
- **Spoken**: CLDR en `temperature-fahrenheit`/`-celsius`/`angle-degree`
  with the plural category ("one degree Celsius", "sixty-eight degrees
  Fahrenheit", "twenty degrees").

## Measure

- **Written**: number plus a CLDR short symbol, optionally compound
  (`10 km`, `120 km/h`, `60 mph`, `5.8 mmol/L`, `mL/min/1.73 m²`,
  `500mg`), or a hyphenated attributive compound (`a 75-g OGTT`).
- **Spoken**: CLDR en long names by plural category ("one kilometer",
  "ten kilometers", "six feet"); `/` → "per" with CLDR's `perUnitPattern`
  form ("kilometers per hour"); a numbered denominator keeps its own
  plural ("per one point seven three square meters"); the hyphenated
  compound takes the singular ("seventy-five-gram").
- **Allow/deny list**: bare `in` (a word), `a`, `s`, `h`, `d`, `N`, `A`,
  `J`, `G`, `t`, `c`, `y`, `mo` are never units after a bare number; they
  still resolve inside compounds (`m/s`) and as prefix bases (`kN`).
  `L`/`l`, `mL`/`ml` both resolve to liters. A bare `G` is giga only
  before `/L`; other `G…` symbols (`Gpt`, `GB`) resolve normally.
- **Digital units**: CLDR `digital-*` (`bit`, `byte`, `KB`/`kB`, `MB`,
  `GB`, `TB`, `PB`, `kbit`, `Mbit`, `Gbit`, `Tbit`): "two gigabytes",
  "one terabyte"; `100 Mbit/s`, `100 Mbps` → "one hundred megabits per
  second".
- **Extra symbols** (no CLDR en unit): `IU`/`IE` international unit(s),
  `kU`, `mU`, `U` unit(s), `Gpt`/`Tpt` giga-/teraparticle(s), `G` giga,
  `mEq` milliequivalent(s), `mosmol` milliosmole(s), `kN` kilonewton(s),
  `mSv` millisievert(s), `dB` decibel(s).
- **Source**: CLDR en `units.json`; NIST SP 811 for symbols.

## Time

- **Written**: `H:MM` with optional `am`/`pm`/`a.m.`/`p.m.` (any case);
  bare hour with a meridiem (`2 pm`); 24-hour `HH:MM` without;
  bare `H:MM:SS` reads as counted nouns ("one hour one minute and one
  second", "fourteen hours ten minutes and thirty seconds"); a meridiem or
  a recognised time zone abbreviation after it makes the reading
  unambiguously a clock instead (see below).
- **Spoken** (**choice**): `9:00` "nine
  o'clock"; `9:05` "nine oh five"; `9:30` "nine thirty"; 24-hour `14:30`
  "fourteen thirty", `14:00` "fourteen hundred", `00:15` "zero fifteen";
  with a meridiem the hour alone for `:00` and the meridiem spelled as
  two letters, "two thirty p m", "two p m", "nine a m" — unambiguous for a
  phonemiser, which may read "pm" as a word. A meridiem glued to a letter
  (`10 pmol`) is not a meridiem; `pm.` at a sentence end keeps its period.
  `H:MM:SS` with a meridiem or a zone reads as the clock plus "and N
  seconds" when nonzero, seconds dropped when zero: `2:30:15 pm` "two
  thirty and fifteen seconds p m", `10:00:00 p.m. EST` "ten p m EST".
- **Time zones**: `2:30 PM EST`, `09:00 UTC` keep the zone as written
  after the spoken time; `10:00 GMT+1` → "ten o'clock GMT plus one"; a
  short all-caps abbreviation (`EST`, `UTC`, `CET`, …) after `H:MM:SS`
  is recognised for this purpose without being parsed itself.
- **Source**: Chicago 9.37–9.39.

## Time range

- **Written**: two times joined by an unspaced dash or " to " (`9:00–11:00`,
  `2–4 pm`, `from 9:00 to 5:00 pm`); bare hours need a meridiem, else they
  are a Range.
- **Spoken**: both ends as Time joined by "to"; a meridiem written once
  applies to both ends and is read once ("two to four p m", "nine to five
  p m"); different meridiems read on each end ("nine a m to five p m").

## Date

- **Written**: numeric `M/D/YYYY` (en-US) or `D/M/YYYY` (en-GB); ISO
  `YYYY-MM-DD`, `YYYY/M/D` (year first is unambiguous in either region)
  and year-month `YYYY-MM` (`2003-03`, zero-padded month 01–12); `Month D[st]`, `Month D, YYYY`, `Mon. D`; `D[st] [of]
  Month [YYYY]`.
- **Spoken**: month name plus ordinal day. Numeric and ISO forms follow
  the region: en-US "November first twenty twenty-six", en-GB "the first
  of November twenty twenty-six". Month-name forms keep their written
  order in both regions: "November first" for `November 1`, "the first of
  November" for `1 November`. The year of `November 1, 2026` is outside
  the span and reads through Year, so the comma stays: "November first,
  twenty twenty-six".
  A year-month is the month name and the year in both regions ("March
  two thousand three").
- **Source**: Chicago 9.31–9.35; ISO 8601; `Options::region`.

## Range

- **Written**: unspaced dash between two numbers or two ordinals (`5–10`,
  `2010–2012`, `1st–3rd`); a spaced dash is left alone.
- **Spoken**: "to" ("five to ten", "twenty ten to twenty twelve", "first
  to third").

## Score

- **Written**: `N:M` that is not a clock time.
- **Spoken** (**choice**): "to" — "two to one". English scores are more
  often written `2–1`, which Range reads the same way.

## Telephone

- **Written**: `+CC …` or a three-digit block in optional parentheses
  followed by separated blocks (`+1 (555) 123-4567`, `(555) 123-4567`,
  `555.123.4567`), or a leading-zero block (`020 7946 0958`); at least
  seven digits in two or more blocks (`555-1234` is a local number,
  `555-123` is a Range).
- **Spoken**: digit by digit, blocks separated by a pause comma, `+` →
  "plus". **Choice**: `0` reads "zero", not "oh" — unambiguous, and
  consistent with Long digit run.
- **Source**: NANP; ITU-T E.123.

## Long digit run

- **Written**: leading-zero runs (`007`), IBAN shapes with bank-code
  letters (`GB29 NWBK 6016 …`), runs beyond 10¹⁸.
- **Spoken**: one character at a time ("zero zero seven", "N W B K"),
  `fallback = true`.
- **Digit groups** (`Token::DigitGroups`): a US SSN `123-45-6789` and a
  16-digit card number `4111 1111 1111 1111` (spaces or hyphens) read digit
  by digit with a pause comma between groups ("one two three, four five,
  six seven eight nine"), `fallback = false`; below IBAN, above Telephone.

## Abbreviation

- **Written**: the table below, with or without inter-word spaces; CLDR
  en abbreviated month and weekday names with a period (`Jan.`, `Mon.`),
  plus `Sept.`.
- **Spoken**: the expansion, only when `Options::expand_abbreviations`.
  `St.` is "Saint" before a capitalised name and "Street" otherwise; `No.`,
  `no.`, `p.`, `pp.`, `vol.`, `ch.`, `fig.` expand only before a number.

| Abbreviation | Expansion |
|---|---|
| e.g. | for example |
| i.e. | that is |
| etc. | et cetera |
| et al. | and others |
| cf. | compare |
| vs. | versus |
| approx. | approximately |
| a.k.a. | also known as |
| Dr. | Doctor |
| Mr. | Mister |
| Mrs. | Missus |
| Prof. | Professor |
| Sr. / Jr. | Senior / Junior |
| St. | Saint (before a name) / Street |
| Mt. | Mount |
| Ave. / Blvd. / Rd. | Avenue / Boulevard / Road |
| No. / no. | Number / number (before a number) |
| Inc. / Ltd. / Co. / Corp. | Incorporated / Limited / Company / Corporation |
| dept. / Dept. | department / Department |
| govt. | government |
| misc. | miscellaneous |
| max. / min. | maximum / minimum |
| fig. / Fig. | figure / Figure (before a number) |
| p. / pp. | page / pages (before a number) |
| vol. / ch. / ed. | volume / chapter / edition |
| est. | established |

Deliberately excluded: `Ms.` (no agreed spelled-out form), `a.m.`/`p.m.`
outside a time (left as written), `U.S.`, `Ph.D.` (letter-by-letter
readings the engine already produces).

## Fractions

- **Written**: `½`, `¼`, `¾`, `⅓`…; `1/2`, `3/4`, `2/3`, `1/8`, `5/12`,
  `1/100`, `1/1000`, `31/32` (13–99: ordinal + "s", "thirty-one
  thirty-seconds"), `1/4th` (the suffix joins the span); mixed `1 ½`, `2 ¾`. Gate as in German: numerator
  below a denominator of 2–12, 100 or 1000; not a date shape (`1/2/2026`
  is a Date in English); not after a unit or currency symbol.
- **Spoken**: numerator as a cardinal, denominator from the hand table
  half/halves, third(s), quarter(s), fifth(s) … twelfth(s), hundredth(s),
  thousandth(s); mixed numbers with "and" and "a" for one ("one and a
  half", "two and three quarters").
- **Priority**: above Year, so `1/1000` is a thousandth (a deviation from
  the German order, harmless: a year can only ever be a denominator).

## Paragraph sign

- `§` → "section", `§§` → "sections"; the number reads via Cardinal.

## Roman numeral (clinical grading)

- As in German: `I`–`IV` word-bounded; a lone `I` only after a grading
  word (Class, Grade, Type, Stage, NYHA, Mallampati, Billroth, Phase,
  Category, Group, Level, Tier, Part, Chapter, Act, Section, Generation,
  Lead, War, Volume, Book), so "I am here" is untouched.

## Scientific notation

- `1.5 × 10⁻⁶`, `1.5e-6`, `1.5E+08` → "one point five times ten to the
  power of minus six"; a trailing unit joins the span ("… per mole",
  "… meters per second"). Same gate as German (`1e10` is not notation).

## Power / exponent

- `x²` "x squared", `x³` "x cubed", otherwise "to the power of" ("two to
  the power of five", "x to the power of minus two"); `m²` after a number
  is Measure ("square meters").

## Chemical formula

- `H₂O` "H two O", `Ca²⁺` "Ca two plus", `SO₄²⁻` "SO four two minus";
  same gates as German.

## Math expression

- `+` plus, spaced `-`/`−` minus, `×`/`·`/`*` times, `÷` divided by, `=`
  equals, `≠` not equal to, `<` less than, `>` greater than, `≤` less than
  or equal to, `≥` greater than or equal to, `≈` approximately, `±` plus or
  minus, `√` the square root of, `∞` infinity, `π` pi. **Choice**: a
  comparison with a left operand reads "is …" ("p is less than zero point
  zero five", "x is approximately three point one four"); the prefix form
  has no "is" ("greater than or equal to sixty-five years").

## Ratio, titer and blood pressure

- `120/80 mmHg`, `BP 120/80` → "over" ("one hundred twenty over eighty
  millimeters of mercury"); `titer 1:80`, `ratio 1:3`, `1 : 3`, `odds 3:1`,
  `scale 1:25,000` → "to"; `100-110/min` → "one hundred to one hundred ten
  per minute". Triggers: titer/titre, ratio, ANA, dilution, scale, odds;
  blood-pressure shorthand `BP` (and `RR`).

## Dose scheme and repetition

- `1-0-1` "one, zero, one", `20-5-0 mg/d` "twenty, five, zero milligrams
  per day". Repetition: `1x` "once", `2x` "twice", `Nx` "N times", a decimal
  count "one point five times"; `2×/day` "twice per day".

## Angle / coordinate

- `30°15′` "thirty degrees fifteen minutes", `52.52° N` "fifty-two point
  five two degrees north"; compass letters N/E/S/W.

## Rendering — as in German

- An integer glued after an uppercase letter gets a space (`B1` → "B
  one", `SpO2` → "SpO two"), and one between letters of any case on both
  sides is spaced on both sides (`H2O` → "H two O", `HbA1c` → "HbA one
  c"); a suffix on a word that starts with the digits stays attached
  (`1stx` → "onestx", `1e10` → "oneeten"); `1.5liters` → "one point
  fiveliters".
