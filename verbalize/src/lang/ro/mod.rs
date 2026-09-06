//! Romanian: gender agreement on 1 and 2 and their
//! compounds, the "de" before a noun from 20 on, ordinals with the
//! `al`/`a` article, and CLDR ro's one/few/other forms.

mod classify;
pub(crate) mod lexicon;
mod verbalize;

use std::collections::HashMap;

use regex::Regex;

use super::units::Units;
use super::{next_word, Context, Lang, Numerals, Tables, Verbalized, SPACE_GROUPS};
use crate::data::LangData;
use crate::pipeline::{self, Match, Recognizers};
use crate::token::{Agreement, Gender, Numeral, Scale, Token};
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

pub(crate) struct Romanian {
    data: &'static LangData,
    units: Units,
    recognizers: Recognizers<Romanian>,
    months: HashMap<&'static str, u8>,
    abbreviations: HashMap<String, (String, String)>,
}

impl Romanian {
    pub(crate) fn new(options: &Options) -> Self {
        let data = Language::Ro.data();
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
        // Months before weekdays: `mar.` is both martie and marți; the month wins.
        for (abbreviated, wide) in data.months_abbreviated.iter().zip(data.months_wide) {
            if abbreviated.ends_with('.') {
                add(abbreviated, wide);
            }
        }
        // `mie.` is left out: "o mie." at a sentence end is the numeral.
        for (abbreviated, wide) in data.weekdays_abbreviated.iter().zip(data.weekdays_wide) {
            if abbreviated.ends_with('.') && *abbreviated != "mie." {
                add(abbreviated, wide);
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

        let mut month_names: Vec<&str> = months.keys().copied().collect();
        month_names.sort_by_key(|m| std::cmp::Reverse(m.len()));
        let month_alternation: Vec<String> = month_names.iter().map(|m| regex::escape(m)).collect();
        Romanian {
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

impl Context for Romanian {
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

    /// CLDR unit names, currency words, the noun lexicon.
    fn noun_gender(&self, word: &str) -> Option<Gender> {
        if let Some(gender) = self.units.gender_of_name(word) {
            return Some(gender);
        }
        if let Some((_, gender)) = lexicon::NOUNS.iter().find(|(noun, _)| *noun == word) {
            return Some(*gender);
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

    fn noun_follows(&self, word: &str) -> bool {
        !lexicon::NOT_A_NOUN.contains(&word)
    }

    /// `22 de ore`: the noun is the word after the written "de".
    fn counted_noun<'a>(&self, text: &'a str, at: usize) -> Option<(&'a str, bool)> {
        let (word, range) = next_word(text, at)?;
        if word == "de" {
            let (noun, _) = next_word(text, range.end)?;
            return Some((noun, true));
        }
        Some((word, false))
    }

    fn scale(&self, word: &str) -> Option<Scale> {
        Some(match word {
            "mii" => Scale::Thousand,
            "mil." | "milion" | "milioane" => Scale::Million,
            "mld." | "miliard" | "miliarde" => Scale::Billion,
            _ => return None,
        })
    }

    fn is_year(&self, year: u64) -> bool {
        (1000..=2999).contains(&year)
    }

    /// Bare `G` is giga only before `/l`.
    fn unit(&self, text: &str, bare: bool) -> Option<crate::token::Unit> {
        if super::common::bare_giga(text)
            && !lexicon::GIGA_CONTEXT.iter().any(|g| text.starts_with(g))
        {
            return None;
        }
        super::common::unit(&self.units, &NUMERALS, text, bare)
    }
}

impl Lang for Romanian {
    fn classify(&self, text: &str) -> Vec<Match> {
        pipeline::classify(&self.recognizers, self, text)
    }

    fn verbalize(&self, token: &Token, agreement: Agreement) -> Verbalized {
        verbalize::verbalize(self, token, agreement)
    }
}
