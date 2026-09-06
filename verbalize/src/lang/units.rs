//! `resolve_unit` (design §7.1): the CLDR short-symbol reverse index, SI
//! prefix composition, and a per-language extra-symbol table and
//! allow/deny list, with the long names by plural category and case.

use fixed_decimal::UnsignedDecimal;
use icu_plurals::{PluralCategory, PluralOperands, PluralRules};
use std::collections::HashMap;

use super::Numerals;
use crate::data::LangData;
use crate::token::{Case, Gender, Numeral, Unit, UnitPart, UnitRef};

/// What a language adds to the CLDR data (design §6.2, §7.6).
pub(crate) struct UnitTable {
    /// Symbol → CLDR unit id, for spellings CLDR's short form does not
    /// use (`mmHg` for `mm Hg`, `min` for `Min.`).
    pub aliases: &'static [(&'static str, &'static str)],
    /// Symbols with no CLDR unit at all.
    pub extra: &'static [ExtraUnit],
    /// Short symbols with several CLDR units: which one wins.
    pub prefer: &'static [(&'static str, &'static str)],
    /// Symbols too ambiguous to stand alone after a number (`s`, `h`);
    /// still resolved inside compounds (`m/s`) and as prefix bases (`kN`).
    pub deny_bare: &'static [&'static str],
    /// The `/` word: "pro".
    pub per: &'static str,
    /// The word CLDR writes between number and noun in some plural forms
    /// (Romanian "{0} de kilometri"); empty when there is none. An SI
    /// prefix attaches to the noun after it ("de picomoli").
    pub link: &'static str,
}

pub(crate) struct ExtraUnit {
    pub symbol: &'static str,
    pub one: &'static str,
    /// The CLDR `few` form where the language has one (Romanian 2–19);
    /// `None` falls back to `other`.
    pub few: Option<&'static str>,
    pub other: &'static str,
    pub gender: Option<Gender>,
}

/// SI prefix symbols and their powers of ten (language-neutral).
const SI_PREFIXES: &[(&str, i8)] = &[
    ("µ", -6),
    ("μ", -6),
    ("n", -9),
    ("p", -12),
    ("m", -3),
    ("c", -2),
    ("d", -1),
    ("h", 2),
    ("k", 3),
    ("M", 6),
    ("G", 9),
    ("T", 12),
];

pub(crate) struct Units {
    data: &'static LangData,
    table: &'static UnitTable,
    by_symbol: HashMap<&'static str, usize>,
    by_id: HashMap<&'static str, usize>,
    /// Every long name form → unit index, the gender lexicon for "ein/eine".
    by_name: HashMap<String, usize>,
    plural: PluralRules,
}

impl Units {
    pub(crate) fn new(data: &'static LangData, table: &'static UnitTable) -> Self {
        let mut by_symbol = HashMap::new();
        let mut by_id = HashMap::new();
        let mut by_name = HashMap::new();
        for (index, unit) in data.units.iter().enumerate() {
            by_id.insert(unit.id, index);
            for pattern in unit.short {
                let symbol = pattern.pattern.replace("{0}", "");
                let symbol = symbol.trim();
                // `{0} km`: the symbol is a suffix of the static pattern.
                let Some(symbol) = pattern
                    .pattern
                    .strip_suffix(symbol)
                    .map(|_| &pattern.pattern[pattern.pattern.len() - symbol.len()..])
                else {
                    continue;
                };
                if symbol.is_empty() {
                    continue;
                }
                match table.prefer.iter().find(|(s, _)| *s == symbol) {
                    Some((_, id)) if *id != unit.id => {}
                    Some(_) => {
                        by_symbol.insert(symbol, index);
                    }
                    None => {
                        by_symbol.entry(symbol).or_insert(index);
                    }
                }
            }
            for pattern in unit.long {
                if let Some(name) = pattern.pattern.strip_prefix("{0}") {
                    by_name.entry(name.trim().to_string()).or_insert(index);
                }
            }
        }
        let locale = icu_locale_core::Locale::try_from_str(data.language)
            .expect("generated data carries a valid language tag");
        let plural = PluralRules::try_new(locale.into(), Default::default())
            .expect("compiled plural data covers every locale (falls back to root)");
        Units {
            data,
            table,
            by_symbol,
            by_id,
            by_name,
            plural,
        }
    }

    /// The `/` word ("pro", "per").
    pub(crate) fn per(&self) -> &'static str {
        self.table.per
    }

    /// Resolves one written symbol. `bare` means it stands alone after a
    /// number (not inside a compound), where the deny list applies.
    pub(crate) fn resolve(&self, symbol: &str, bare: bool) -> Option<UnitRef> {
        if let Some(extra) = self.table.extra.iter().find(|e| e.symbol == symbol) {
            return Some(UnitRef::Extra(extra.symbol));
        }
        if bare && self.table.deny_bare.contains(&symbol) {
            return None;
        }
        if let Some(id) = self.cldr_id(symbol) {
            return Some(UnitRef::Cldr { id, prefix: None });
        }
        for (prefix, power) in SI_PREFIXES {
            if let Some(base) = symbol.strip_prefix(prefix) {
                if !base.is_empty() {
                    if let Some(id) = self.cldr_id(base) {
                        return Some(UnitRef::Cldr {
                            id,
                            prefix: Some(*power),
                        });
                    }
                }
            }
        }
        None
    }

    fn cldr_id(&self, symbol: &str) -> Option<&'static str> {
        if let Some((_, id)) = self.table.aliases.iter().find(|(s, _)| *s == symbol) {
            return Some(id);
        }
        let index = *self.by_symbol.get(symbol)?;
        Some(self.data.units[index].id)
    }

    /// Data-rate spellings that fold the `/s` into the symbol; the same in
    /// every language, resolved as their `A/s` compound.
    const COMPOUND_ALIASES: &'static [(&'static str, &'static str)] = &[
        ("bps", "bit/s"),
        ("kbps", "kbit/s"),
        ("Kbps", "kbit/s"),
        ("Mbps", "Mbit/s"),
        ("Gbps", "Gbit/s"),
        ("Tbps", "Tbit/s"),
    ];

    /// Splits `text` at `/` and resolves every segment; the first may be
    /// empty (`/min`), a denominator may carry its own number (`1,73 m²`).
    pub(crate) fn resolve_compound(
        &self,
        numerals: &Numerals,
        text: &str,
        bare: bool,
    ) -> Option<Unit> {
        let text = Self::COMPOUND_ALIASES
            .iter()
            .find(|(written, _)| *written == text)
            .map_or(text, |(_, canonical)| canonical);
        let mut segments = text.split('/');
        let first = segments.next()?;
        let numerator = if first.is_empty() {
            None
        } else {
            Some(UnitPart {
                factor: None,
                unit: self.resolve(first, bare)?,
            })
        };
        let mut denominators = Vec::new();
        for segment in segments {
            denominators.push(self.parse_part(numerals, segment)?);
        }
        if numerator.is_none() && denominators.is_empty() {
            return None;
        }
        Some(Unit {
            numerator,
            denominators,
        })
    }

    /// A denominator segment: `min`, `1,73 m²`.
    fn parse_part(&self, numerals: &Numerals, segment: &str) -> Option<UnitPart> {
        let symbol_at = segment
            .find(|c: char| !(c.is_ascii_digit() || c == numerals.decimal))
            .unwrap_or(segment.len());
        let (number, symbol) = segment.split_at(symbol_at);
        let symbol = symbol.trim_start();
        if symbol.is_empty() {
            return None;
        }
        let factor = if number.is_empty() {
            None
        } else {
            Some(numerals.parse(number)?)
        };
        Some(UnitPart {
            factor,
            unit: self.resolve(symbol, false)?,
        })
    }

    pub(crate) fn gender(&self, unit: &UnitRef) -> Option<Gender> {
        match unit {
            UnitRef::Cldr { id, .. } => self.data.units[self.by_id[id]].gender,
            UnitRef::Extra(symbol) => self.extra(symbol).gender,
            UnitRef::Word(_) => None,
        }
    }

    /// Gender of a unit named by one of its long forms (`Litern`, `Stunde`).
    pub(crate) fn gender_of_name(&self, word: &str) -> Option<Gender> {
        self.by_name
            .get(word)
            .and_then(|&i| self.data.units[i].gender)
    }

    /// The long name for `n` in `case`: `Kilometer`, `Kilometern`, `Millimol`.
    pub(crate) fn name(&self, unit: &UnitRef, n: &Numeral, case: Case) -> String {
        let category = self.category(n);
        match unit {
            UnitRef::Cldr { id, prefix } => {
                let cldr = &self.data.units[self.by_id[id]];
                let pattern = cldr
                    .long
                    .iter()
                    .find(|p| p.plural == category && p.case == case)
                    .or_else(|| {
                        cldr.long
                            .iter()
                            .find(|p| p.plural == category && p.case == Case::Nominative)
                    })
                    .or_else(|| {
                        cldr.long.iter().find(|p| {
                            p.plural == PluralCategory::Other && p.case == Case::Nominative
                        })
                    })
                    .map(|p| p.pattern)
                    .or(cldr.long_name)
                    .unwrap_or(cldr.id);
                self.prefixed(strip_placeholder(pattern), *prefix)
            }
            UnitRef::Extra(symbol) => {
                let extra = self.extra(symbol);
                match category {
                    PluralCategory::One => extra.one,
                    PluralCategory::Few => extra.few.unwrap_or(extra.other),
                    _ => extra.other,
                }
                .to_string()
            }
            UnitRef::Word(word) => word.clone(),
        }
    }

    /// The name after "pro": `Liter`, `Quadratmeter`.
    pub(crate) fn per_name(&self, unit: &UnitRef) -> String {
        match unit {
            UnitRef::Cldr { id, prefix } => {
                let cldr = &self.data.units[self.by_id[id]];
                let per = format!("{{0}} {} ", self.table.per);
                let name = match cldr.long_per.and_then(|p| p.strip_prefix(per.as_str())) {
                    Some(name) => name.to_string(),
                    None => return self.name(unit, &Numeral::int(1), Case::Nominative),
                };
                self.prefixed(&name, *prefix)
            }
            _ => self.name(unit, &Numeral::int(1), Case::Nominative),
        }
    }

    /// `Milli{0}` + `Mol` → `Millimol`.
    fn prefixed(&self, name: &str, prefix: Option<i8>) -> String {
        let Some(power) = prefix else {
            return name.to_string();
        };
        let Some((_, pattern)) = self.data.unit_prefixes.iter().find(|(p, _)| *p == power) else {
            return name.to_string();
        };
        // The prefix attaches to the noun, after a linking word ("de picomoli").
        let link = format!("{} ", self.table.link);
        let (linker, noun) = match name.strip_prefix(&link) {
            Some(noun) if !self.table.link.is_empty() => (link.as_str(), noun),
            _ => ("", name),
        };
        let mut chars = noun.chars();
        let lowered = match chars.next() {
            Some(first) => first.to_lowercase().chain(chars).collect::<String>(),
            None => String::new(),
        };
        format!("{linker}{}", pattern.replace("{0}", &lowered))
    }

    fn extra(&self, symbol: &str) -> &'static ExtraUnit {
        self.table
            .extra
            .iter()
            .find(|e| e.symbol == symbol)
            .expect("extra symbol from this table")
    }

    /// Plural category of the written form: `1` is "one", `1,0` and `1,5`
    /// are not (CLDR operands `v`/`f` count the visible fraction digits).
    pub(crate) fn category(&self, n: &Numeral) -> PluralCategory {
        if n.fraction.is_empty() {
            let operands = PluralOperands::from(u64::try_from(n.integer).unwrap_or(u64::MAX));
            return self.plural.category_for(operands);
        }
        let written = format!("{}.{}", n.integer, n.fraction);
        let operands = match UnsignedDecimal::try_from_str(&written) {
            Ok(decimal) => PluralOperands::from(&decimal),
            Err(_) => PluralOperands::from(0u64),
        };
        self.plural.category_for(operands)
    }
}

fn strip_placeholder(pattern: &str) -> &str {
    match pattern.strip_prefix("{0}") {
        Some(rest) => rest.trim(),
        None => pattern.trim_end_matches("{0}").trim(),
    }
}
