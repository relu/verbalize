//! CLDR/ICU rule-based number format (RBNF), formatting direction only.
//!
//! A port of the structure of ICU4J's `RuleBasedNumberFormat`, `NFRuleSet`,
//! `NFRule` and `NFSubstitution` (design §6.4, §8): the same rule
//! selection (largest base value ≤ n, radix/exponent divisors, the rollback
//! rule), the same bracket splitting into two rules, the same substitution
//! semantics. Differences: numbers are integers plus a decimal digit
//! string, never floating point; rule syntax the vendored locales do not use
//! is rejected at parse time rather than interpreted loosely; output has
//! soft hyphens removed and whitespace collapsed.
//!
//! [`parse`] builds the compiled representation, [`format`] evaluates it.

mod decimal;
mod format;
mod parse;
#[cfg(test)]
mod tests;

use std::fmt;

use icu_plurals::{PluralCategory, PluralRules};

use crate::data::LangData;
use decimal::DecimalPattern;

/// A number to spell: sign, integer magnitude and the decimal digits after
/// the point. `fraction` is spoken verbatim, trailing zeros included
/// ("3,10" → "drei Komma eins null"), so the caller decides what a written
/// fraction means; ICU, holding a `double`, would drop them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Value<'a> {
    pub negative: bool,
    pub integer: u128,
    /// ASCII digits, possibly empty. Anything else makes `format` return `None`.
    pub fraction: &'a str,
}

impl Value<'static> {
    pub(crate) fn int(n: i128) -> Self {
        Value {
            negative: n < 0,
            integer: n.unsigned_abs(),
            fraction: "",
        }
    }
}

impl<'a> Value<'a> {
    /// Zero is never negative.
    pub(crate) fn decimal(negative: bool, integer: u128, fraction: &'a str) -> Self {
        let negative = negative && (integer != 0 || !fraction.is_empty());
        Value {
            negative,
            integer,
            fraction,
        }
    }

    fn is_fraction(self) -> bool {
        !self.fraction.is_empty()
    }
}

/// The locale's number symbols the interpreter needs: the decimal separator
/// selects between `x.x` and `x,x` rules, the group separator and minus
/// sign render `=#,##0=` fallbacks.
#[derive(Clone, Debug)]
pub(crate) struct Symbols {
    decimal: String,
    group: String,
    minus: String,
}

impl Symbols {
    pub(crate) fn new(decimal: &str, group: &str, minus: &str) -> Self {
        Symbols {
            decimal: decimal.into(),
            group: group.into(),
            minus: minus.into(),
        }
    }
}

/// Compiled rulesets of one locale. Built once per language and shared.
pub(crate) struct Rbnf {
    rulesets: Vec<RuleSet>,
    /// Index of the first public ruleset, ICU's default.
    default: usize,
    symbols: Symbols,
    /// Plural rules, built only when a rule uses `$(cardinal,…)$` / `$(ordinal,…)$`.
    cardinal: Option<PluralRules>,
    ordinal: Option<PluralRules>,
}

impl Rbnf {
    pub(crate) fn new(data: &'static LangData) -> Result<Self, ParseError> {
        let s = &data.symbols;
        let symbols = Symbols::new(s.decimal, s.group, s.minus);
        Rbnf::compile(
            data.language,
            symbols,
            data.rbnf.iter().map(|r| (r.name, r.rules)),
        )
    }

    /// Name of the default ruleset (the first public one).
    pub(crate) fn default_ruleset(&self) -> &str {
        &self.rulesets[self.default].name
    }

    /// Spells `n` with the named ruleset (`%spellout-numbering`; private
    /// `%%` names are accepted too). `None` when the ruleset does not exist
    /// or the rules cannot format the value, which validated CLDR data does
    /// not do for values in range.
    pub(crate) fn format(&self, ruleset: &str, n: Value<'_>) -> Option<String> {
        let index = self.rulesets.iter().position(|r| r.name == ruleset)?;
        if !n.fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut out = String::new();
        self.format_value(index, n, &mut out, 0).ok()?;
        Some(postprocess(&out))
    }
}

/// Soft hyphens (U+00AD, throughout CLDR German) removed, whitespace runs
/// collapsed to one space, ends trimmed.
fn postprocess(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut space = false;
    for c in raw.chars() {
        if c == '\u{ad}' {
            continue;
        }
        if c.is_whitespace() {
            space = !out.is_empty();
        } else {
            if space {
                out.push(' ');
                space = false;
            }
            out.push(c);
        }
    }
    out
}

struct RuleSet {
    name: String,
    /// Normal rules in non-decreasing base-value order.
    rules: Vec<Rule>,
    /// `-x`
    negative: Option<Rule>,
    /// `x.x`
    improper: Option<Rule>,
    /// `0.x`
    proper: Option<Rule>,
    /// `x.0`
    master: Option<Rule>,
}

struct Rule {
    base: u128,
    /// `radix^exponent`; 1 for the special rules.
    divisor: u128,
    /// Rule text and substitutions in source order; ICU stores the text
    /// with the substitution positions, which is the same thing.
    pieces: Vec<Piece>,
    has_modulus: bool,
}

enum Piece {
    Text(String),
    Sub(Substitution),
    Plural(PluralFormat),
}

struct Substitution {
    kind: SubKind,
    target: Target,
}

/// ICU's `NFSubstitution` subclasses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubKind {
    /// `<<` in a normal rule: `n / divisor`.
    Multiplier,
    /// `>>` in a normal rule: `n % divisor`.
    Modulus,
    /// `>>>`: `n % divisor` formatted directly by the previous rule.
    Predecessor(usize),
    /// `==`-family: the same value, elsewhere.
    SameValue,
    /// `<<` in a fraction rule: the integer part.
    IntegralPart,
    /// `>>` in a fraction rule: the fraction, one digit at a time.
    FractionalPart { spaces: bool },
    /// `>>` in the negative rule: the magnitude.
    AbsoluteValue,
}

enum Target {
    RuleSet(usize),
    Decimal(DecimalPattern),
}

/// `$(cardinal,one{st}other{th})$`
struct PluralFormat {
    ordinal: bool,
    forms: Vec<(PluralCategory, String)>,
}

/// Rule syntax outside the interpreter's scope, or an inconsistent ruleset.
/// Carries the ruleset name and the offending rule text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParseError {
    pub ruleset: String,
    pub rule: String,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rbnf: {}: in {} rule {:?}",
            self.message, self.ruleset, self.rule
        )
    }
}

impl std::error::Error for ParseError {}
