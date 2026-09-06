//! `verbalize survey <file|dir|sqlite:path>`.
//!
//! Extracts every token containing a digit or a classified symbol from a
//! corpus, groups by "shape" (digits mapped to `9`, everything else kept),
//! and reports frequency plus current coverage.

use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::Path;

use verbalize::{Language, Normalizer};

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

/// Whitespace-delimited words with their byte ranges.
fn word_spans(text: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, i, &text[s..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, text.len(), &text[s..]));
    }
    out
}

fn overlaps(range: &Range<usize>, start: usize, end: usize) -> bool {
    start < range.end && range.start < end
}

#[derive(Clone)]
pub struct ShapeEntry {
    pub shape: String,
    pub count: usize,
    pub example: String,
    pub spoken: String,
    pub unhandled: bool,
}

pub struct Report {
    pub all: Vec<ShapeEntry>,
    pub unhandled: Vec<ShapeEntry>,
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
            unhandled: false,
        });
        entry.count += 1;
        if span.fallback {
            entry.unhandled = true;
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
            unhandled: true,
        });
        entry.count += 1;
        entry.unhandled = true;
    }
}

fn survey_dir(
    dir: &Path,
    normalizer: &Normalizer,
    shapes: &mut HashMap<String, ShapeEntry>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            survey_dir(&path, normalizer, shapes)?;
        } else if let Ok(text) = fs::read_to_string(&path) {
            survey_text(&text, normalizer, shapes);
        }
        // Non-UTF-8/binary files are silently skipped: this is a text corpus tool.
    }
    Ok(())
}

fn survey_sqlite(
    path: &str,
    normalizer: &Normalizer,
    shapes: &mut HashMap<String, ShapeEntry>,
) -> Result<(), String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| format!("opening {path}: {e}"))?;

    let mut tables_stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| e.to_string())?;
    let tables: Vec<String> = tables_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    drop(tables_stmt);

    for table in tables {
        let pragma = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        let mut cols_stmt = conn.prepare(&pragma).map_err(|e| e.to_string())?;
        let columns: Vec<String> = cols_stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let ty: String = row.get(2)?;
                Ok((name, ty))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|(_, ty)| ty.to_ascii_uppercase().contains("TEXT"))
            .map(|(name, _)| name)
            .collect();
        drop(cols_stmt);

        for column in columns {
            let query = format!(
                "SELECT \"{}\" FROM \"{}\"",
                column.replace('"', "\"\""),
                table.replace('"', "\"\"")
            );
            let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, Option<String>>(0))
                .map_err(|e| e.to_string())?;
            for value in rows {
                if let Some(text) = value.map_err(|e| e.to_string())? {
                    survey_text(&text, normalizer, shapes);
                }
            }
        }
    }
    Ok(())
}

pub fn run(path: &str) -> Result<Report, String> {
    let normalizer = Normalizer::new(Language::De);
    let mut shapes: HashMap<String, ShapeEntry> = HashMap::new();

    if let Some(db_path) = path.strip_prefix("sqlite:") {
        survey_sqlite(db_path, &normalizer, &mut shapes)?;
    } else {
        let p = Path::new(path);
        if p.is_dir() {
            survey_dir(p, &normalizer, &mut shapes)?;
        } else {
            let text = fs::read_to_string(p).map_err(|e| format!("reading {path}: {e}"))?;
            survey_text(&text, &normalizer, &mut shapes);
        }
    }

    let mut all: Vec<ShapeEntry> = shapes.into_values().collect();
    all.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.shape.cmp(&b.shape)));
    let unhandled: Vec<ShapeEntry> = all.iter().filter(|s| s.unhandled).cloned().collect();

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
    println!("shape\tcount\texample\tspoken");
    for entry in &report.unhandled {
        println!(
            "{}\t{}\t{}\t{}",
            entry.shape, entry.count, entry.example, entry.spoken
        );
    }
}
