# Integration guide

How to call `verbalize` from a TTS pipeline (design §10). This document
expands that section for integrators; the design itself is the source of
truth for behaviour.

## Where to call it

`Normalizer::normalize` is a pure `&str → String` pass. Call it once per
utterance, **before** sentence splitting, chunking, or any other streaming
concern — never per line, never per already-split sentence:

```rust
use std::sync::LazyLock;
use verbalize::{Language, Normalizer};

static DE: LazyLock<Normalizer> = LazyLock::new(|| Normalizer::new(Language::De));

fn speak(utterance: &str) -> String {
    DE.normalize(utterance)
}
```

Construction compiles the language's recognisers once and is the expensive
step (measured below): keep exactly one `Normalizer` per language for the
process lifetime, e.g. behind a `LazyLock` as above, or one per language in
a small registry if you serve more than one. `Normalizer` is `Send + Sync`
and safe to share across request threads without additional locking.

Naive sentence splitters treat "z. B.", "Sa.," and "19.30 Uhr" as sentence
ends, because they all contain periods that look like full stops. After
normalisation those periods are gone ("zum Beispiel", "Samstag", "neunzehn
Uhr dreißig"), so splitting after normalising sidesteps the whole class of
bug. Splitting first and normalising each fragment independently is not
equivalent — a classifier needs the surrounding sentence for agreement
(design §7.3) and for multi-word classes (dates, time ranges) that a
premature split would sever.

## Local phonemiser engines (Piper / espeak-ng)

Local engines built on `espeak-ng` (Piper and most on-device engines) have
a language-generic digit reader with no idea that "1.000" is one thousand
in German, or that "8,80 €" is a price. They need the full expansion:
keep `Options::expand_abbreviations` on (the default) so the phonemiser
never has to guess at raw digits, symbols, or abbreviations.

## Hosted neural engines

Neural end-to-end APIs accept raw text and often guess correctly in
English, but read German conventions inconsistently — the same input can
come out different ways on different calls, with nothing to configure.
Normalising before the call makes their reading deterministic and
identical to the local path, since both engines now receive the same
already-spelled-out text. Punctuation and casing outside classified spans
are preserved byte-for-byte (design §9), which keeps the prosody cues these
engines rely on ("Sie ruft Sie um 3:00 Uhr an." keeps its comma, capital
letters and full stop; only "3:00 Uhr" itself changes).

## Domain lexicon

`Options::lexicon` is a literal, case-sensitive, word-bounded substitution
list applied *after* every semiotic class, so a domain entry can only
replace letters the digit-bearing classes left untouched — it can never
shadow a phone number, dose, or ratio span. This is how an integration adds
vocabulary the German-language rules deliberately don't know about
(clinical shorthand, product codes, internal abbreviations):

```rust
use verbalize::{Language, Normalizer, Options};

let options = Options {
    lexicon: vec![
        ("SpO2".into(), "Sauerstoffsättigung".into()),
        ("RR".into(), "Blutdruck".into()),
    ],
    ..Options::default()
};
let de = Normalizer::with_options(Language::De, options);
assert_eq!(de.normalize("SpO2 92 %"), "Sauerstoffsättigung zweiundneunzig Prozent");
```

(Verified via the CLI: `verbalize normalize --lexicon lexicon.tsv` with a
`SpO2<TAB>Sauerstoffsättigung` line produces exactly that output — see the
README.) The CLI reads the same list from a `literal<TAB>spoken` TSV file
via `--lexicon`; a library caller builds the `Vec<(String, String)>`
directly.

## Coverage in CI: `verbalize survey`

Run `verbalize survey` over the actual text your application sends to TTS
— not a synthetic sample — and fail the build when a new, frequent,
unhandled shape shows up:

```sh
# export the corpus your app actually speaks, one utterance per line or
# one row per table if it comes from a database, then:
verbalize survey exported-utterances/ --threshold 5
```

The exit code is non-zero when any shape in the "unhandled" section
(`fallback = true`, or a digit/symbol token no span covered at all) occurs
at least `--threshold` times. This is the mechanism that finds the next
"8,80 €" — a shape the classifier doesn't yet know — before a user hears it
misread, rather than after. Point it at a directory of exported text files
or a `sqlite:<path>` database (every `TEXT` column of every table is
scanned); wire it into the same CI that runs your test suite so a coverage
regression fails the build the same way a broken test would.

## Idempotence

`normalize(normalize(t)) == normalize(t)` is a hard property (design §9),
enforced by a property test over every fixture input and over random
digit-rich strings (`cargo test -p verbalize --test fixtures`). The output
of `normalize` never contains an ASCII digit or a classified symbol, so
running it twice — for example if your pipeline both normalises for a
local phonemiser and again defensively before a remote fallback — is
always a safe no-op, not a source of double-reading bugs.

## Performance

Measured with `cargo bench -p verbalize` (`verbalize/benches/normalize.rs`,
release build, this machine — treat as orders-of-magnitude guidance, not a
guaranteed SLA):

| Benchmark | Median |
|---|---|
| Normalize 5,000 chars of ordinary prose (a number or two per paragraph) | 731 µs |
| Normalize 5,000 chars, number-dense (clinical/financial text: every sentence has money, times, ratios, doses, …) | 1.99 ms |
| Normalize 5,000 chars with no numbers at all | 81.5 µs |
| `Normalizer::new(Language::De)` (recogniser + RBNF compilation) | 37.2 ms |

The last row is why construction must be cached (§"Where to call it"
above): compiling the German recognisers and RBNF rulesets once costs tens
of milliseconds, but calling `normalize` on that already-built `Normalizer`
costs well under a millisecond even for number-dense paragraphs, and under
100 µs when there is nothing to classify. Design §12's target — a
5,000-character paragraph normalising in well under a millisecond — holds
for ordinary and numberless prose; the number-dense worst case is
currently a low single-digit number of milliseconds and is not gated by
this design's stated target, since §12 does not specify a number-dense
scenario separately.
