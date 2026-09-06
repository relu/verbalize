//! English (design §7.4): the opposite conventions to German — `1,000.50`,
//! `$8.80`, 12-hour times, `1st`, month-day-year (en-US) or day-month-year
//! (en-GB) selected by `Options::region`.

mod classify;
pub(crate) mod lexicon;
mod verbalize;

use std::collections::HashMap;
use std::ops::Range;

use regex::Regex;

use super::de::followed_by_name;
use super::units::Units;
use super::{Context, Lang, Numerals, Tables, Verbalized, SPACE_GROUPS};
use crate::data::LangData;
use crate::pipeline::{self, Match, Recognizers};
use crate::token::{Agreement, Gender, NounPosition, Numeral, Scale, Token};
use crate::{Language, Options, Region};

pub(super) const NUMERALS: Numerals = Numerals {
    groups: &[
        ',',
        SPACE_GROUPS[0],
        SPACE_GROUPS[1],
        SPACE_GROUPS[2],
        SPACE_GROUPS[3],
    ],
    decimal: '.',
};

pub(crate) struct English {
    data: &'static LangData,
    units: Units,
    recognizers: Recognizers<English>,
    /// Day before month in numeric dates and "the first of November" (en-GB).
    day_first: bool,
    months: HashMap<&'static str, u8>,
    abbreviations: HashMap<String, (String, String)>,
}

impl English {
    pub(crate) fn new(options: &Options) -> Self {
        let data = Language::En.data();
        let mut months = HashMap::new();
        for (i, name) in data.months_wide.iter().enumerate() {
            months.insert(*name, i as u8 + 1);
        }
        for (i, name) in data.months_abbreviated.iter().enumerate() {
            months.insert(*name, i as u8 + 1);
        }
        months.insert("Sept", 9);

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
        // CLDR's abbreviated names carry no period; the written form does.
        for (abbreviated, wide) in data.weekdays_abbreviated.iter().zip(data.weekdays_wide) {
            add(&format!("{abbreviated}."), wide);
        }
        for (abbreviated, wide) in data.months_abbreviated.iter().zip(data.months_wide) {
            if *abbreviated != wide {
                add(&format!("{abbreviated}."), wide);
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
        English {
            data,
            units: Units::new(data, &lexicon::UNIT_TABLE),
            recognizers: Recognizers::new(classify::recognizers(
                &month_alternation.join("|"),
                abbreviation_regex,
            )),
            day_first: options.region == Some(Region::EnGb),
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

impl Context for English {
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

    fn words(&self, n: &Numeral, _: Option<Gender>) -> String {
        verbalize::words(n)
    }

    /// Any bare four-digit number from 1000 reads as a year ("ten sixty-six").
    fn scale(&self, word: &str) -> Option<Scale> {
        Some(match word {
            "thousand" | "k" => Scale::Thousand,
            "million" | "M" => Scale::Million,
            "billion" | "bn" => Scale::Billion,
            "trillion" => Scale::Trillion,
            _ => return None,
        })
    }

    fn is_year(&self, year: u64) -> bool {
        (1000..=2999).contains(&year)
    }

    /// Bare `G` is giga only before `/l` (§7.6).
    fn unit(&self, text: &str, bare: bool) -> Option<crate::token::Unit> {
        if super::common::bare_giga(text)
            && !lexicon::GIGA_CONTEXT.iter().any(|g| text.starts_with(g))
        {
            return None;
        }
        super::common::unit(&self.units, &NUMERALS, text, bare)
    }

    /// `St.` is "Saint" before a name (`NounPosition::After`), `No.` only
    /// before a number, `p.`/`pp.` only before a number.
    fn abbreviation_context(
        &self,
        key: &str,
        text: &str,
        range: &Range<usize>,
    ) -> Option<Agreement> {
        let mut agreement = Agreement::default();
        match key {
            "St." if followed_by_name(self, text, range) => agreement.noun = NounPosition::After,
            "No." | "no." | "p." | "pp." | "vol." | "ch." | "fig." | "Fig." => {
                let rest = text[range.end..].trim_start();
                if !rest.starts_with(|c: char| c.is_ascii_digit()) {
                    return None;
                }
            }
            _ => {}
        }
        Some(agreement)
    }
}

impl Lang for English {
    fn classify(&self, text: &str) -> Vec<Match> {
        pipeline::classify(&self.recognizers, self, text)
    }

    fn verbalize(&self, token: &Token, agreement: Agreement) -> Verbalized {
        verbalize::verbalize(self, token, agreement)
    }
}
