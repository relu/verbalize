# Romanian (`ro`) — rule provenance

One section per semiotic class for the Romanian module, the proving
ground of the agreement layer. Readings no source fixes are
marked **choice**; every reading has a fixture in
`verbalize/tests/fixtures/ro/`.

Sources: DOOM3 (Dicționarul ortografic, ortoepic și morfologic al limbii
române, 3rd ed., 2021) for numeral forms and abbreviations; Gramatica
Academiei (Gramatica limbii române, vol. I–II) for numeral agreement, the
"de" construction and ordinal formation; SR ISO 8601 and the Romanian
Standards Institute conventions for numeric dates; CLDR ro (`rbnf.json`,
`units.json`, `currencies.json`, `ca-gregorian.json`).

## Cardinal

- **Written**: `.` or space grouping (`1.000`, `22 000`), decimal comma.
- **Spoken**: CLDR ro RBNF, gendered by the noun that follows:
  `%spellout-cardinal-masculine` / `-feminine` / `-neuter` when the gender
  is known from a CLDR unit name, a currency word or the noun lexicon
  (`lexicon.rs`: ani, persoane, ore, zile, tablete, milioane, mii …);
  `%spellout-numbering` otherwise. A bare 1 before a noun takes the
  article form "un"/"o" ("un leu", "o oră"); "unu"/"una" stand alone.
  Compounds agree ("douăzeci și două de ore", "douăsprezece ore").
- **"de" insertion** (Gramatica Academiei I §4.2.3): before the counted
  noun from 20 on — exactly when the CLDR plural category is `other`
  (`few` covers 2–19 and every number ending in 01–19, so "o sută unu lei"
  but "o sută douăzeci de lei"). The verbalizer appends "de" to a Cardinal
  whose following word is a noun (`NounPosition::After`, set by the
  classifier unless the word is in the function-word stoplist or "de" is
  already written); CLDR's own `other` unit patterns carry it ("{0} de
  kilometri"); currencies, fractions and repetition add it in code.
- **Thousands**: from 1000 up the groups are composed in the verbalizer —
  count (feminine for "mii", neuter for "milioane"), "de" by the count's
  category, noun singular/plural — because CLDR ro's RBNF writes "douăzeci
  și două mii" without the "de" DOOM3 requires ("douăzeci și două de mii",
  "două sute de mii", "un milion două sute de mii").
- **Standard forms**: CLDR ro spells the accepted variants "patrusprezece",
  "șasesprezece", "șasezeci", "una sută", "una mie", "unu milion"; DOOM3
  gives "paisprezece", "șaisprezece", "șaizeci", "o sută", "o mie", "un
  milion" as the standard, and every number word is rewritten to them
  (`STANDARD_FORMS`). `spell::ruleset` still returns CLDR's raw form, which
  is what `tools/icu-diff` compares.

## Year

- Bare 1000–2999 reads through the year ruleset, which in Romanian is the
  cardinal ("o mie nouă sute nouăzeci", "două mii douăzeci și patru"). No
  decade suffix exists (`anii '90` is out of scope).

## Ordinal

- **Written** (DOOM3): `al N-lea` (masculine), `a N-a` (feminine), with the
  article optional in the text and Roman numerals allowed (`al II-lea`,
  `a XX-a`, `sec. al XX-lea`); `1-a`/`al 1-lea` for the first.
- **Spoken**: "primul"/"prima" for 1; otherwise the article plus the
  cardinal with the ordinal ending on its last word (Gramatica Academiei I
  §4.3): masculine `-lea` after a vowel, `-ulea` after a consonant ("al
  doilea", "al treilea", "al optulea", "al douăzeci și unulea", "al o
  sutălea"); feminine "a doua", "a treia", "a patra", "a cincea", "a
  opta", "a noua", "a douăzecea", "a douăzeci și una". CLDR ro has no
  ordinal rulesets, so this derivation is the source.
- A bare `5.` is a Cardinal followed by a period (Romanian does not mark
  ordinals with a period).

## Decimal

- Comma; the integer part agrees with a following noun of known gender
  ("două virgulă cinci ore"), fraction digits one by one.

## Dotted number

- **Written**: digit groups joined by periods that are not a grouped
  Cardinal (`1.000`) nor a Date (`1.11.2026`): `2.3`, `3.10`, `1.2.7`.
- **Spoken**: each group as a Cardinal joined by "punct" ("doi punct
  trei", "unu punct doi punct șapte"); a leading-zero group digit by digit
  ("patru punct zero doi"). Previously the period was left between the
  two number words.

## Electronic

- **Written**: as in German — `@` addresses, `scheme://` URLs, `www.…`,
  bare domains with a known TLD (`example.ro`, `example.com`), IPv4,
  `@handles`; `de` and `eu` are dropped from the TLD list for Romanian
  (they are words), so `etc.`, `dr.`, `1.000`, `1.11.2026` never match.
- **Spoken**: letters as written, digits one by one, `@` "at", `.` "punct",
  `/` "bară", `:` "două puncte", `?` "semnul întrebării", `=` "egal", `&`
  "și", `-` "liniuță", `_` "linie jos", `#` "diez", `+` "plus", `~`
  "tildă", `%` "la sută" (**choice**, common spoken names):
  `ion.popescu@example.ro` → "ion punct popescu at example punct ro",
  `https://www.example.ro/cale?x=1` → "https două puncte bară bară www
  punct example punct ro bară cale semnul întrebării x egal unu".

## Money

- **Written**: `1,50 lei`, `20 de lei`, `20 lei`, `100 RON`, `20 de
  euro`, `1,50 €`, `5 $`, `10 dolari`, `£5`.
- **Spoken**: head noun is the first word of the CLDR display name by
  plural category ("leu"/"lei", "dolar"/"dolari", "liră"/"lire", "euro");
  "de" from 20 on; major and minor joined with "și", minor unit from the
  hand table ban/bani, cent/cenți, penny/pence ("un leu și cincizeci de
  bani", "doi lei și cinci bani", "cincizeci de bani"); `,00` dropped;
  three or more fraction digits read as a decimal plus the currency.
- **Scale words**: `mii`, `mil.`/`milion`/`milioane`, `mld.`/`miliard`/
  `miliarde` between amount and currency; the scale noun takes the
  count's "de" from 20 on and the currency always follows with "de"
  (DOOM3): `3 mil. lei` → "trei milioane de lei", `2,5 mld. lei` → "două
  virgulă cinci miliarde de lei", `1 mil. euro` → "un milion de euro". No
  currency → `Scaled` with "de" before a following noun (`3 milioane
  locuitori` → "trei milioane de locuitori"; a written "de" is kept).
- **Source**: CLDR ro `currencies.json`; BNR conventions for leu/ban.

## Percent, permille

- "la sută", "la mie" (**choice** over CLDR's "procente": the spoken norm).
  No "de" ("douăzeci la sută").

## Degrees, Measure

- CLDR ro long names with one/few/other ("un grad Celsius", "două grade",
  "douăzeci de grade Celsius"; "un kilometru", "doi kilometri", "douăzeci
  de kilometri"); `/` → "pe" from CLDR's `perUnitPattern` ("kilometri pe
  oră"). `mp` is an alias for square meters ("metri pătrați"). Extra
  symbols carry their own few/other forms ("unități"/"de unități";
  `Gpt`/`Tpt` "gigaparticule"/"teraparticule"). Bare `h` and `m` are
  accepted after a number (hours, meters); `s`, `d`, `N`, `A`, `J`, `G`,
  `t`, `a`, `c` are not; a bare `G` is giga only before `/l`, other `G…`
  symbols resolve normally.
  Digital units from CLDR `digital-*` (`bit`, `byte`, `KB`/`kB`, `MB`,
  `GB`, `TB`, `Mbit`, `Gbit`): "doi gigabyți", "un terabyte", `100 Mbit/s`
  and `100 Mbps` → "o sută de megabiți pe secundă".

## Time

- **Written**: `HH:MM`, `H:MM`, `ora H`.
- **Spoken** (**choice**):
  `:00` → "ora paisprezece", "ora nouă"; otherwise "paisprezece și
  treizeci", "nouă și cinci"; `ora 9` → "ora nouă". Hours are feminine
  ("ora două", "ora douăsprezece", "douăzeci și două și cincisprezece")
  except 1 and 21 ("ora unu", "ora douăzeci și unu").
- Time range: both readings joined by "până la".
- Time zones stay as written after the time (`14:30 EET`); `12:00 UTC+2`
  → "ora douăsprezece UTC plus doi".

## Date

- **Written**: `D luna [YYYY]`, `DD.MM.YYYY`, `D.M.`, ISO `YYYY-MM-DD` and
  year-month `YYYY-MM` (`2003-03`, zero-padded month 01–12).
- **Spoken**: day as a masculine cardinal ("doi noiembrie", "douăzeci și
  patru decembrie"), "întâi" for the first ("întâi noiembrie două mii
  douăzeci și șase"); month name from CLDR ro (lower case); year as above.
  A year-month is the month and the year ("martie două mii trei").

## Range, Score

- Range: "până la" ("cinci până la zece minute"). Before a noun the range
  agrees like a lone cardinal: the noun's gender reaches both ends ("două
  până la trei ore") and the end next to the noun takes the "de" from 20
  on (DOOM3; "douăzeci până la treizeci de minute", "o sută până la două
  sute de minute"); a written "de" is kept as is ("20–30 de minute").
- Score: "la" (**choice**) — "doi la unu".

## Telephone, Long digit run

- Romanian shapes: `+40 21 123 4567`, `0721 123 456`, `021/1234567`,
  `(021) 123 4567` (parenthesised area code); digit
  by digit with pause commas, "plus" for `+`, "zero" for 0. IBANs
  (`RO49 AAAA 1B31 …`) one character at a time, letters included.
- Digit groups: a 16-digit card number (`4111 1111 1111 1111`) digit by
  digit with a pause between groups, like a telephone number.

## Abbreviation

- DOOM3 list (`lexicon.rs`): etc. → etcetera, ex. → exemplu (so `de ex.` →
  "de exemplu"), nr. → numărul, dr. → doctor, str. → strada, bd. →
  bulevardul, aprox. → aproximativ, sec. → secolul, d.Hr./î.Hr. →
  după/înainte de Hristos, pag. → pagina, art. → articolul, alin. →
  alineatul, cca. → circa, ș.a. → și altele, ș.a.m.d. → și așa mai
  departe, resp. → respectiv, cf. → conform, vs. → versus, dl./dna. →
  domnul/doamna, prof. → profesor, ing. → inginer, jud. → județul, com. →
  comuna, tel. → telefon; capitalised variants where they occur in text.
- CLDR ro month abbreviations (`ian.`, `sept.`, `dec.`) and weekday
  abbreviations (`lun.`, `vin.`) with two exceptions: `mar.` is both martie
  and marți and expands to the month; `mie.` (miercuri) is left out because
  "o mie." at a sentence end is the numeral.

## Fractions

- Feminine fraction nouns (jumătate/jumătăți, treime/treimi, cincime …,
  sutime/sutimi, miime/miimi) with "un sfert"/"sferturi" neuter: "o
  jumătate", "două treimi", "trei sferturi", "o optime", "douăzeci de
  sutimi"; mixed numbers with "și" ("unu și jumătate", "doi și trei
  sferturi"). Denominators 13–99 derive the noun from the cardinal minus
  its final vowel plus "-ime"/"-imi" ("o treisprezecime", "trei
  douăzecimi").

## Paragraph sign, Roman numerals

- `§` → "paragraful", `§§` → "paragrafele". Roman I–IV in grading contexts
  as in German (Clasa, Gradul, Tipul, Stadiul, NYHA, …); `sec. XX` is not
  expanded (V and above are out of scope), `sec. al XX-lea` is an Ordinal.

## Scientific, power, chemical, math, ratio, dose, repetition, angle (**choice** of standard usage)

- Scientific: "unu virgulă cinci ori zece la puterea minus șase" (+ unit).
- Power: "la pătrat", "la cub", otherwise "la puterea".
- Chemical: "H doi O", "Ca doi plus".
- Math: `+` plus, spaced `-` minus, `×`/`·`/`*` ori, `÷` împărțit la, `=`
  egal, `≠` diferit de, `<` mai mic decât, `>` mai mare decât, `≤` mai mic
  sau egal cu, `≥` mai mare sau egal cu, `≈` aproximativ, `±` plus minus,
  `√` radical din, `∞` infinit, `π` pi; `%` "la sută".
- Ratio: blood pressure `120/80 mmHg`, `TA 120/80` → "cu" ("o sută
  douăzeci cu optzeci de milimetri coloană de mercur"); titer/ratio/scale
  `1:640` → "la"; `100-110/min` → "până la … pe minut". Triggers: titru,
  raport, ANA, diluție, scara/scară, proporție; shorthand TA (and RR, BP).
- Dose scheme: "unu, zero, unu", "douăzeci, cinci, zero miligrame pe zi".
- Repetition: "o dată", "de două ori", "de trei ori", "de douăzeci de
  ori" (feminine "ori" takes the "de" rule), decimals "de unu virgulă cinci
  ori"; `2×/zi` → "de două ori pe zi".
- Angle: "treizeci de grade cincisprezece minute douăzeci de secunde
  nord"; compass N/E/S/V.

## Core hooks Romanian relies on

- `Context::counted_noun` (default: the next word) lets a language look
  past its linking word (`22 de ore`) and tell the verbalizer the word is
  already written; `Context::noun_follows` (default: always) excludes
  function words. `common::cardinal` records the noun as
  `NounPosition::After` from these.
- `ExtraUnit` gained a `few` form (`None` in de/en) and `UnitTable` a
  `link` word so an SI prefix attaches after CLDR's "de" ("de picomoli").
- `Tables::decade_suffix` may be empty.
