//! `verbalize audit <file|dir|sqlite:path>`.
//!
//! Finds the misreading class where a unit symbol is guessed as an SI prefix
//! on a shorter symbol (`Grad` = `G` + `rad`): for every span read with a
//! composed unit it takes the written unit word, then reports whether that
//! same word also occurs in the corpus outside any span. A token that is both
//! is a word the corpus uses as prose, so the composition guess is suspect.

use std::collections::HashMap;

use verbalize::token::{Token, Unit, UnitRef};
use verbalize::{Language, Normalizer};

use crate::corpus::{covered, for_each_text, word_spans};

pub struct Entry {
    pub symbol: String,
    pub composed: usize,
    /// Occurrences of the same word outside any classified span.
    pub word: usize,
    pub example: String,
}

pub struct Report {
    /// Composed-unit tokens, most word-like first.
    pub entries: Vec<Entry>,
}

impl Report {
    /// Composed-unit tokens the corpus also uses as an ordinary word.
    pub fn suspicious(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.word > 0)
    }
}

fn is_composed(unit: &Unit) -> bool {
    unit.numerator.iter().chain(&unit.denominators).any(|part| {
        matches!(
            part.unit,
            UnitRef::Cldr {
                prefix: Some(_),
                ..
            }
        )
    })
}

/// Whether a span's unit was guessed as an SI prefix on a shorter symbol.
fn composed_unit(token: &Token) -> bool {
    match token {
        Token::Measure { unit, .. } => is_composed(unit),
        Token::Range {
            unit: Some(unit), ..
        } => is_composed(unit),
        Token::Repetition {
            per: Some(unit), ..
        } => is_composed(unit),
        _ => false,
    }
}

/// The written unit word of a span: its last whitespace-delimited token,
/// without a trailing abbreviation dot (`Min.` → `Min`).
fn unit_word(written: &str) -> Option<&str> {
    let last = written.split_whitespace().next_back()?;
    let word = last.trim_end_matches('.');
    (!word.is_empty()).then_some(word)
}

fn audit_text(
    text: &str,
    normalizer: &Normalizer,
    composed: &mut HashMap<String, (usize, String)>,
    words: &mut HashMap<String, usize>,
) {
    let spans = normalizer.annotate(text);
    for span in &spans {
        if !composed_unit(&span.token) {
            continue;
        }
        let written = &text[span.range.clone()];
        if let Some(word) = unit_word(written) {
            let entry = composed
                .entry(word.to_string())
                .or_insert_with(|| (0, written.to_string()));
            entry.0 += 1;
        }
    }
    let mut cursor = 0;
    for (start, end, word) in word_spans(text) {
        if covered(&spans, &mut cursor, start, end) {
            continue;
        }
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if word.chars().any(char::is_alphabetic) {
            *words.entry(word.to_string()).or_default() += 1;
        }
    }
}

pub fn run(path: &str, language: Language) -> Result<Report, String> {
    let normalizer = Normalizer::new(language);
    let mut composed: HashMap<String, (usize, String)> = HashMap::new();
    let mut words: HashMap<String, usize> = HashMap::new();
    for_each_text(path, &mut |text| {
        audit_text(text, &normalizer, &mut composed, &mut words)
    })?;

    let mut entries: Vec<Entry> = composed
        .into_iter()
        .map(|(symbol, (composed, example))| Entry {
            word: words.get(&symbol).copied().unwrap_or(0),
            symbol,
            composed,
            example,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.word
            .cmp(&a.word)
            .then_with(|| b.composed.cmp(&a.composed))
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    Ok(Report { entries })
}

pub fn print_report(report: &Report) {
    println!("# composed units that also occur as ordinary words");
    println!("symbol\tcomposed\tas_word\texample");
    for entry in report.suspicious() {
        println!(
            "{}\t{}\t{}\t{}",
            entry.symbol, entry.composed, entry.word, entry.example
        );
    }
    println!();
    println!("# composed units, no ordinary-word use in this corpus");
    println!("symbol\tcomposed\texample");
    for entry in report.entries.iter().filter(|e| e.word == 0) {
        println!("{}\t{}\t{}", entry.symbol, entry.composed, entry.example);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verbalize::token::{Numeral, Unit, UnitPart};

    fn measure(prefix: Option<i8>) -> Token {
        Token::Measure {
            value: Numeral::int(29),
            unit: Unit {
                numerator: Some(UnitPart {
                    factor: None,
                    unit: UnitRef::Cldr {
                        id: "angle-radian",
                        prefix,
                    },
                }),
                denominators: Vec::new(),
            },
            hyphenated: false,
        }
    }

    #[test]
    fn a_guessed_prefix_is_composed_but_a_plain_symbol_is_not() {
        assert!(composed_unit(&measure(Some(9))));
        assert!(!composed_unit(&measure(None)));
    }

    #[test]
    fn unit_word_is_the_last_token_without_its_dot() {
        assert_eq!(unit_word("29 Grad"), Some("Grad"));
        assert_eq!(unit_word("5-10 Min."), Some("Min"));
        assert_eq!(unit_word("3 kcal"), Some("kcal"));
    }
}
