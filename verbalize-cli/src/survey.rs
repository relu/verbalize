//! `verbalize survey <file|dir|sqlite:path>`.
//!
//! Extracts every token containing a digit or a classified symbol from a
//! corpus, groups by "shape" (digits mapped to `9`, everything else kept),
//! and reports frequency plus current coverage.

use std::collections::HashMap;

use verbalize::{Language, Normalizer};

use crate::corpus::{for_each_text, overlaps, word_spans};

/// Symbols the semiotic classes recognise besides plain digits: currency,
/// percent/permille, degree, paragraph, math/scientific operators, Unicode
/// fractions and primes.
const SYMBOLS: &[char] = &[
    '€', '$', '£', '%', '‰', '°', '§', '×', '·', '±', '≤', '≥', '≠', '≈', '√', '½', '¼', '¾', '′',
    '″',
];

fn is_digit_like(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(c,
            '\u{2070}'..='\u{2079}' | '\u{00B9}' | '\u{00B2}' | '\u{00B3}' | '\u{2080}'..='\u{2089}')
}

fn contains_target(s: &str) -> bool {
    s.chars().any(|c| is_digit_like(c) || SYMBOLS.contains(&c))
}

/// digits (ASCII or super/subscript) → `9`, everything else kept as written.
fn shape_of(s: &str) -> String {
    s.chars()
        .map(|c| if is_digit_like(c) { '9' } else { c })
        .collect()
}

#[derive(Clone)]
pub struct ShapeEntry {
    pub shape: String,
    /// Total occurrences seen.
    pub count: usize,
    /// An occurrence the classifier claimed.
    pub example: String,
    pub spoken: String,
    /// Occurrences left verbatim or read as a last resort.
    pub unhandled: usize,
    pub unhandled_example: String,
    pub unhandled_spoken: String,
}

pub struct Report {
    pub all: Vec<ShapeEntry>,
    pub unhandled: Vec<ShapeEntry>,
}

/// Record one unhandled occurrence, keeping the first as the example.
fn note_unhandled(entry: &mut ShapeEntry, example: &str, spoken: &str) {
    if entry.unhandled == 0 {
        entry.unhandled_example = example.to_string();
        entry.unhandled_spoken = spoken.to_string();
    }
    entry.unhandled += 1;
}

fn survey_text(text: &str, normalizer: &Normalizer, shapes: &mut HashMap<String, ShapeEntry>) {
    let spans = normalizer.annotate(text);
    for span in &spans {
        let original = &text[span.range.clone()];
        if !contains_target(original) {
            continue;
        }
        let shape = shape_of(original);
        let entry = shapes.entry(shape.clone()).or_insert_with(|| ShapeEntry {
            shape,
            count: 0,
            example: original.to_string(),
            spoken: span.spoken.clone(),
            unhandled: 0,
            unhandled_example: String::new(),
            unhandled_spoken: String::new(),
        });
        entry.count += 1;
        if span.fallback {
            note_unhandled(entry, original, &span.spoken);
        }
    }

    for (start, end, word) in word_spans(text) {
        if spans.iter().any(|s| overlaps(&s.range, start, end)) {
            continue;
        }
        if !contains_target(word) {
            continue;
        }
        let shape = shape_of(word);
        let entry = shapes.entry(shape.clone()).or_insert_with(|| ShapeEntry {
            shape,
            count: 0,
            example: word.to_string(),
            spoken: word.to_string(),
            unhandled: 0,
            unhandled_example: String::new(),
            unhandled_spoken: String::new(),
        });
        entry.count += 1;
        note_unhandled(entry, word, word);
    }
}

pub fn run(path: &str, language: Language) -> Result<Report, String> {
    let normalizer = Normalizer::new(language);
    let mut shapes: HashMap<String, ShapeEntry> = HashMap::new();
    for_each_text(path, &mut |text| {
        survey_text(text, &normalizer, &mut shapes)
    })?;

    let mut all: Vec<ShapeEntry> = shapes.into_values().collect();
    all.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.shape.cmp(&b.shape)));
    let unhandled: Vec<ShapeEntry> = all.iter().filter(|s| s.unhandled > 0).cloned().collect();

    Ok(Report { all, unhandled })
}

pub fn print_report(report: &Report) {
    println!("shape\tcount\texample\tspoken");
    for entry in &report.all {
        println!(
            "{}\t{}\t{}\t{}",
            entry.shape, entry.count, entry.example, entry.spoken
        );
    }
    println!();
    println!("# unhandled (fallback or left verbatim)");
    println!("shape\tunhandled\tcount\texample\tspoken");
    for entry in &report.unhandled {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            entry.shape,
            entry.unhandled,
            entry.count,
            entry.unhandled_example,
            entry.unhandled_spoken
        );
    }
}
