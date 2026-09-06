//! CLDR tables, compiled from the JSON vendored under `xtask/cldr/` by
//! `cargo xtask gen`. This file is hand-written; the per-language files and
//! `supplemental.rs` are generated and must not be edited. `cargo xtask
//! check-data` fails when they are stale.
//!
//! Everything is `static` data with `&'static str` leaves: no parsing, no
//! allocation, no I/O at runtime.

use crate::token::{Case, Gender};
use icu_plurals::PluralCategory;

pub mod de;
pub mod en;
pub mod ro;
pub mod supplemental;

/// Generated CLDR tables for one language.
pub struct LangData {
    /// BCP-47 language subtag, e.g. `de`.
    pub language: &'static str,
    /// Symbols of the `latn` numbering system (`numbers.json`).
    pub symbols: NumberSymbols,
    /// RBNF `SpelloutRules` rulesets in CLDR file order (`rbnf/<lang>.json`).
    /// The first public ruleset is ICU's default.
    pub rbnf: &'static [Ruleset],
    /// Currencies in CLDR order, i.e. by ISO 4217 code (`currencies.json`).
    pub currencies: &'static [Currency],
    /// Minor-unit digits by ISO 4217 code, ascending (supplemental
    /// `currencyData`); codes not listed use `currency_digits_default`.
    pub currency_digits: &'static [(&'static str, u8)],
    pub currency_digits_default: u8,
    /// Measurement units in CLDR order (`units.json`), excluding prefixes and
    /// compound patterns.
    pub units: &'static [Unit],
    /// SI prefix patterns by power of ten, e.g. `(3, "Kilo{0}")`, `(-6, "Mikro{0}")`.
    pub unit_prefixes: &'static [(i8, &'static str)],
    /// Gregorian month names, format context, January first (`ca-gregorian.json`).
    pub months_wide: [&'static str; 12],
    pub months_abbreviated: [&'static str; 12],
    /// Gregorian weekday names, format context, Sunday first (CLDR order).
    pub weekdays_wide: [&'static str; 7],
    pub weekdays_abbreviated: [&'static str; 7],
}

pub struct NumberSymbols {
    pub decimal: &'static str,
    pub group: &'static str,
    pub percent: &'static str,
    pub permille: &'static str,
    pub minus: &'static str,
}

/// One RBNF ruleset, e.g. `%spellout-ordinal-n` or the private `%%ste`.
pub struct Ruleset {
    pub name: &'static str,
    /// `(descriptor, body)` pairs verbatim from CLDR, in file order, e.g.
    /// `("1100/100", "←←\u{ad}hundert[\u{ad}→→];")`. The RBNF interpreter
    /// parses these; soft hyphens are kept as CLDR wrote them.
    pub rules: &'static [(&'static str, &'static str)],
}

pub struct Currency {
    /// ISO 4217 code, e.g. `EUR`.
    pub code: &'static str,
    /// `displayName`, e.g. `Euro`.
    pub name: &'static str,
    /// `displayName-count-*`, e.g. `[(One, "Britisches Pfund"), (Other, "Britische Pfund")]`.
    pub names: &'static [(PluralCategory, &'static str)],
    /// `symbol`, e.g. `€`; absent when CLDR uses the code.
    pub symbol: Option<&'static str>,
    /// `symbol-alt-narrow`, often ambiguous (`$` for every dollar).
    pub symbol_narrow: Option<&'static str>,
    /// `symbol-alt-variant`.
    pub symbol_variant: Option<&'static str>,
}

pub struct Unit {
    /// CLDR unit identifier, e.g. `length-kilometer`.
    pub id: &'static str,
    /// Grammatical gender of the long form, where CLDR has it.
    pub gender: Option<Gender>,
    /// Long `displayName`, e.g. `Kilometer`.
    pub long_name: Option<&'static str>,
    /// Long `perUnitPattern`, e.g. `{0} pro Kilometer`: the denominator form.
    pub long_per: Option<&'static str>,
    /// Long patterns by plural category and case, e.g. `{0} Kilometern`.
    pub long: &'static [UnitPattern],
    /// Short `displayName`, e.g. `km`.
    pub short_name: Option<&'static str>,
    /// Short patterns by plural category, e.g. `{0} km`; the source of the
    /// symbol → unit reverse index.
    pub short: &'static [UnitPattern],
}

/// A unit pattern verbatim from CLDR: `{0}` is the number. Kept verbatim
/// because the number is not always first (`B {0}` for Beaufort) and some
/// case forms carry an article (`unui kilometru`).
pub struct UnitPattern {
    pub plural: PluralCategory,
    /// `Nominative` for the untagged `unitPattern-count-*` form.
    pub case: Case,
    pub pattern: &'static str,
}
