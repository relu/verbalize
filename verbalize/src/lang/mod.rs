//! The language layer: per-language classification and
//! verbalization behind one trait, plus the building blocks every language
//! shares — the number format, digit-boundary checks, the CLDR unit
//! resolver and the language-neutral recognisers in [`common`].

pub(crate) mod common;
pub(crate) mod de;
pub(crate) mod en;
pub(crate) mod ro;

pub(crate) mod units;

use std::ops::Range;

use crate::pipeline::Match;
use crate::token::{Agreement, Gender, Numeral, Scale, Token, Unit};
use crate::{Language, Options};

/// `classify` returns matches with their agreement rather than bare spans,
/// so the context the classifier saw reaches the verbalizer; `data()` is
/// not needed by the pipeline and is omitted.
pub(crate) trait Lang: Send + Sync {
    /// Typed, non-overlapping matches in position order; nothing spoken yet.
    fn classify(&self, text: &str) -> Vec<Match>;
    fn verbalize(&self, token: &Token, agreement: Agreement) -> Verbalized;
}

pub(crate) struct Verbalized {
    pub spoken: String,
    pub fallback: bool,
}

impl Verbalized {
    pub(crate) fn plain(spoken: String) -> Self {
        Verbalized {
            spoken,
            fallback: false,
        }
    }
}

pub(crate) fn new(language: Language, options: &Options) -> Box<dyn Lang> {
    match language {
        Language::De => Box::new(de::German::new(options)),
        Language::En => Box::new(en::English::new(options)),
        Language::Ro => Box::new(ro::Romanian::new(options)),
    }
}

/// What the shared recognisers ask a language for: a language module is
/// mostly tables and a recogniser list.
pub(crate) trait Context: Send + Sync + 'static {
    fn numerals(&self) -> &Numerals;
    fn units(&self) -> &units::Units;
    fn tables(&self) -> &'static Tables;
    /// Month number of a month name, wide or abbreviated.
    fn month(&self, word: &str) -> Option<u8>;
    /// The abbreviation table key for an abbreviation as written.
    fn abbreviation_key(&self, written: &str) -> Option<&str>;
    /// Cardinal words for `n`, with the gendered "ein/eine" when the
    /// language inflects and the gender is known.
    fn words(&self, n: &Numeral, gender: Option<Gender>) -> String;
    /// Gender of a noun a cardinal governs; languages without
    /// agreement return `None`.
    fn noun_gender(&self, _word: &str) -> Option<Gender> {
        None
    }
    /// Whether the word after a cardinal is the noun it counts (in
    /// Romanian, "de" is inserted before it from 20 on); function words are not.
    fn noun_follows(&self, _word: &str) -> bool {
        true
    }
    /// The word a cardinal ending at `at` counts, and whether the
    /// language's linking word is already written between them
    /// (`22 de ore`), so the verbalizer does not add it again.
    fn counted_noun<'a>(&self, text: &'a str, at: usize) -> Option<(&'a str, bool)> {
        next_word(text, at).map(|(word, _)| (word, false))
    }
    /// Bare four-digit numbers in this range read as years.
    fn is_year(&self, year: u64) -> bool;
    /// Resolves a written unit, with the language's own gates on top of
    /// the resolver's; `X⁻¹` is a bare rate.
    fn unit(&self, text: &str, bare: bool) -> Option<Unit> {
        common::unit(self.units(), self.numerals(), text, bare)
    }
    /// Accepts an abbreviation match with the agreement it carries, for
    /// languages whose expansion depends on context (`Fr.` → Frau/Freitag,
    /// `No.` only before a number); `None` rejects the match.
    fn abbreviation_context(
        &self,
        _key: &str,
        _text: &str,
        _range: &Range<usize>,
    ) -> Option<Agreement> {
        Some(Agreement::default())
    }
    /// The magnitude a written scale word denotes (`Mio.`, `billion`,
    /// `mld.`); `None` when `word` is not one in this language.
    fn scale(&self, _word: &str) -> Option<Scale> {
        None
    }
}

/// Hand tables the shared recognisers and verbalizers read.
pub(crate) struct Tables {
    /// Words before `N:M` that make it a ratio.
    pub ratio_triggers: &'static [&'static str],
    /// The blood-pressure shorthand before `N/M` (`RR`, `BP`).
    pub blood_pressure: &'static [&'static str],
    pub currencies: &'static [Currency],
    /// Slash-fraction denominators: value → name (singular, plural).
    pub fractions: &'static [(u32, &'static str, &'static str)],
    pub unicode_fractions: &'static [(char, u32, u32)],
    /// Compass letters after a degree value → direction word.
    pub compass: &'static [(char, &'static str)],
    /// Words after which a lone Roman `I` is the numeral one.
    pub roman_context: &'static [&'static str],
    /// "plus"/"minus" for charges and exponents.
    pub plus: &'static str,
    pub minus: &'static str,
    /// The decade suffix glued to a year (`1990er`, `1990s`).
    pub decade_suffix: &'static str,
    /// The word between the groups of a dotted number (`2.3` → "zwei Punkt drei").
    pub point: &'static str,
    /// The spoken names of the symbols in an email, URL or handle
    /// (`@` "at", `.` "Punkt", `/` "Schrägstrich", …).
    pub electronic: &'static [(char, &'static str)],
    /// Top-level domains that are ordinary words of the language (`de`,
    /// `eu` in Romanian; `at`, `it` in English): a bare `name.tld` with one
    /// of these is not a domain, so glued prose (`zero.de cinci`) is never
    /// misread and the output stays idempotent.
    pub tld_words: &'static [&'static str],
}

/// Currencies Money recognises: ISO code, written forms, head-noun gender
/// (for "ein Euro"), minor unit singular/plural with gender.
pub(crate) struct Currency {
    pub code: &'static str,
    pub written: &'static [&'static str],
    pub gender: Option<Gender>,
    pub minor: (&'static str, &'static str, Option<Gender>),
}

/// How the language writes numbers: group separators and the decimal
/// separator (`1.000,5` vs `1,000.5`).
pub(crate) struct Numerals {
    pub groups: &'static [char],
    pub decimal: char,
}

impl Numerals {
    /// Regex fragment: a grouped integer (`1.000`, `22 000`) or a plain run.
    pub(crate) fn int(&self) -> String {
        let class: String = self
            .groups
            .iter()
            .map(|c| regex::escape(&c.to_string()))
            .collect();
        format!("(?:[0-9]{{1,3}}(?:[{class}][0-9]{{3}})+|[0-9]+)")
    }

    /// Regex fragment: an integer with an optional fraction.
    pub(crate) fn num(&self) -> String {
        format!(
            "{}(?:{}[0-9]+)?",
            self.int(),
            regex::escape(&self.decimal.to_string())
        )
    }

    /// Regex fragment: a plain number with an optional fraction (no grouping).
    pub(crate) fn plain(&self) -> String {
        format!(
            "[0-9]+(?:{}[0-9]+)?",
            regex::escape(&self.decimal.to_string())
        )
    }

    /// Parses `[-]INT[<decimal>FRAC]` as written; `None` on overflow or
    /// stray characters.
    pub(crate) fn parse(&self, text: &str) -> Option<Numeral> {
        let (negative, rest) = match text.strip_prefix(['-', '−']) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (int, fraction) = rest.split_once(self.decimal).unwrap_or((rest, ""));
        if !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        Some(Numeral {
            negative,
            integer: self.parse_int(int)?,
            fraction: fraction.to_string(),
        })
    }

    /// Parses a written integer with group separators; `None` on overflow.
    pub(crate) fn parse_int(&self, digits: &str) -> Option<u128> {
        let mut value = 0u128;
        for c in digits.chars() {
            if let Some(d) = c.to_digit(10) {
                value = value.checked_mul(10)?.checked_add(u128::from(d))?;
            } else if !self.groups.contains(&c) {
                return None;
            }
        }
        Some(value)
    }
}

/// Whitespace group separators DIN 5008 and the Chicago style share.
pub(crate) const SPACE_GROUPS: [char; 4] = [' ', '\u{a0}', '\u{202f}', '\u{2009}'];

// ------------------------------------------------------------- helpers

/// The digits of `range` are a whole digit run: no ASCII digit touches it
/// on either side. Every recogniser checks this so a run is never split.
pub(crate) fn digit_bounded(text: &str, range: &Range<usize>) -> bool {
    let before = text.as_bytes()[..range.start].last();
    let after = text.as_bytes()[range.end..].first();
    !before.is_some_and(u8::is_ascii_digit) && !after.is_some_and(u8::is_ascii_digit)
}

/// The character before `at`, if any.
pub(crate) fn char_before(text: &str, at: usize) -> Option<char> {
    text[..at].chars().next_back()
}

pub(crate) fn char_after(text: &str, at: usize) -> Option<char> {
    text[at..].chars().next()
}

/// `range` is not glued to a letter on either side.
pub(crate) fn letter_free(text: &str, range: &Range<usize>) -> bool {
    !char_before(text, range.start).is_some_and(char::is_alphabetic)
        && !char_after(text, range.end).is_some_and(char::is_alphabetic)
}

/// The word (letters only) starting at `at` after optional whitespace, with
/// its byte range.
pub(crate) fn next_word(text: &str, at: usize) -> Option<(&str, Range<usize>)> {
    let rest = &text[at..];
    let start = at + rest.len() - rest.trim_start().len();
    let word: &str = text[start..].split(|c: char| !c.is_alphabetic()).next()?;
    (!word.is_empty()).then(|| (word, start..start + word.len()))
}

/// The word (letters only) ending right before `at`, after optional whitespace.
pub(crate) fn previous_word(text: &str, at: usize) -> Option<&str> {
    let head = text[..at].trim_end();
    let word = head.rsplit(|c: char| !c.is_alphabetic()).next()?;
    (!word.is_empty()).then_some(word)
}
