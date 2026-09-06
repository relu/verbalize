//! German (design §7.2, §7.3, §7.6).

mod classify;
pub(crate) mod lexicon;
mod verbalize;

use std::collections::HashMap;
use std::ops::Range;

use regex::Regex;

use super::units::Units;
use super::{next_word, Context, Lang, Numerals, Tables, Verbalized, SPACE_GROUPS};
use crate::data::LangData;
use crate::pipeline::{self, Match, Recognizers};
use crate::token::{Agreement, Gender, NounPosition, Numeral, Scale, Token};
use crate::{Language, Options};

pub(super) const NUMERALS: Numerals = Numerals {
    groups: &[
        '.',
        SPACE_GROUPS[0],
        SPACE_GROUPS[1],
        SPACE_GROUPS[2],
        SPACE_GROUPS[3],
    ],
    decimal: ',',
};

pub(crate) struct German {
    data: &'static LangData,
    units: Units,
    recognizers: Recognizers<German>,
    /// Month name (wide or abbreviated) → month number.
    months: HashMap<&'static str, u8>,
    /// Whitespace-stripped abbreviation → (as written in the table, expansion),
    /// plus CLDR weekday and month abbreviations.
    abbreviations: HashMap<String, (String, String)>,
}

impl German {
    pub(crate) fn new(options: &Options) -> Self {
        let data = Language::De.data();
        let mut months = HashMap::new();
        for (i, name) in data.months_wide.iter().enumerate() {
            months.insert(*name, i as u8 + 1);
        }
        for (i, name) in data.months_abbreviated.iter().enumerate() {
            months.insert(*name, i as u8 + 1);
        }

        let mut abbreviations: HashMap<String, (String, String)> = HashMap::new();
        let mut add = |key: &str, expansion: &str| {
            let stripped: String = key.chars().filter(|c| !c.is_whitespace()).collect();
            abbreviations
                .entry(stripped)
                .or_insert_with(|| (key.to_string(), expansion.to_string()));
        };
        for (key, expansion) in lexicon::ABBREVIATIONS {
            add(key, expansion);
        }
        for (abbreviated, wide) in data.weekdays_abbreviated.iter().zip(data.weekdays_wide) {
            add(abbreviated, wide);
        }
        for (i, wide) in data.months_wide.iter().enumerate() {
            let abbreviated = data.months_abbreviated[i];
            if abbreviated.ends_with('.') {
                add(abbreviated, wide);
            }
            // The three-letter form (`Sep.`) beside CLDR's own (`Sept.`).
            let three: String = wide.chars().take(3).collect();
            if three != *wide {
                add(&format!("{three}."), wide);
            }
        }
        let abbreviation_regex = options.expand_abbreviations.then(|| {
            let mut keys: Vec<&String> = abbreviations.values().map(|(k, _)| k).collect();
            keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
            let alternation: Vec<String> = keys
                .iter()
                .map(|k| regex::escape(k).replace(' ', "\\s?"))
                .collect();
            Regex::new(&format!("(?:{})", alternation.join("|"))).expect("escaped keys")
        });

        // Longest first: `November` must win over `Nov` in the alternation.
        let mut month_names: Vec<&str> = months.keys().copied().collect();
        month_names.sort_by_key(|m| std::cmp::Reverse(m.len()));
        let month_alternation: Vec<String> = month_names.iter().map(|m| regex::escape(m)).collect();
        German {
            data,
            units: Units::new(data, &lexicon::UNIT_TABLE),
            recognizers: Recognizers::new(classify::recognizers(
                &month_alternation.join("|"),
                abbreviation_regex,
            )),
            months,
            abbreviations,
        }
    }

    fn expansion(&self, key: &str) -> Option<&str> {
        let stripped: String = key.chars().filter(|c| !c.is_whitespace()).collect();
        self.abbreviations
            .get(&stripped)
            .map(|(_, expansion)| expansion.as_str())
    }
}

impl Context for German {
    fn numerals(&self) -> &Numerals {
        &NUMERALS
    }

    fn units(&self) -> &Units {
        &self.units
    }

    fn tables(&self) -> &'static Tables {
        &lexicon::TABLES
    }

    fn month(&self, word: &str) -> Option<u8> {
        self.months.get(word).copied()
    }

    fn abbreviation_key(&self, written: &str) -> Option<&str> {
        let stripped: String = written.chars().filter(|c| !c.is_whitespace()).collect();
        self.abbreviations
            .get(&stripped)
            .map(|(key, _)| key.as_str())
    }

    fn words(&self, n: &Numeral, gender: Option<Gender>) -> String {
        verbalize::words(n, gender)
    }

    /// Gender of a noun the number governs: CLDR unit names, the ordinal
    /// lexicon, currency head nouns (§7.3).
    fn noun_gender(&self, word: &str) -> Option<Gender> {
        if let Some(gender) = self.units.gender_of_name(word) {
            return Some(gender);
        }
        if let Some(gender) = self.lexicon_noun(word) {
            return Some(gender);
        }
        for currency in lexicon::TABLES.currencies {
            if currency.written.contains(&word) && word.chars().all(char::is_alphabetic) {
                return currency.gender;
            }
            if word == currency.minor.0 || word == currency.minor.1 {
                return currency.minor.2;
            }
        }
        None
    }

    /// Bare 1100–1999 is a year (§7.2); 2000–2999 read the same either way.
    /// The full words also lower-case (`2 millionen km/h` in loose prose).
    fn scale(&self, word: &str) -> Option<Scale> {
        Some(match word {
            "Tsd." | "Tausend" | "tausend" => Scale::Thousand,
            "Mio." | "Mio" | "Million" | "Millionen" | "million" | "millionen" => Scale::Million,
            "Mrd." | "Milliarde" | "Milliarden" | "milliarde" | "milliarden" => Scale::Billion,
            "Bio." | "Billion" | "Billionen" | "billion" | "billionen" => Scale::Trillion,
            _ => return None,
        })
    }

    fn is_year(&self, year: u64) -> bool {
        (1100..=2999).contains(&year)
    }

    /// Bare `G` is Giga only before `/l` (§7.6).
    fn unit(&self, text: &str, bare: bool) -> Option<crate::token::Unit> {
        if super::common::bare_giga(text) && !text.starts_with(lexicon::GIGA_CONTEXT) {
            return None;
        }
        super::common::unit(&self.units, &NUMERALS, text, bare)
    }

    /// "Fr." is "Frau" before a capitalised name (`NounPosition::After`),
    /// "Freitag" otherwise.
    fn abbreviation_context(
        &self,
        key: &str,
        text: &str,
        range: &Range<usize>,
    ) -> Option<Agreement> {
        let mut agreement = Agreement::default();
        if key == "Fr." && followed_by_name(self, text, range) {
            agreement.noun = NounPosition::After;
        }
        Some(agreement)
    }
}

/// A capitalised word follows after one space, and it is not itself an
/// abbreviation (`Fr. Müller`, `St. John`).
pub(crate) fn followed_by_name<C: Context>(c: &C, text: &str, range: &Range<usize>) -> bool {
    let Some((word, _)) = next_word(text, range.end) else {
        return false;
    };
    word.starts_with(char::is_uppercase)
        && c.abbreviation_key(&format!("{word}.")).is_none()
        && text[range.end..].starts_with(' ')
}

impl Lang for German {
    fn classify(&self, text: &str) -> Vec<Match> {
        pipeline::classify(&self.recognizers, self, text)
    }

    fn verbalize(&self, token: &Token, agreement: Agreement) -> Verbalized {
        verbalize::verbalize(self, token, agreement)
    }
}
