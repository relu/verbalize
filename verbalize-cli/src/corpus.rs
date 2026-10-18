//! Corpus walking shared by `survey` and `audit`: hand every text of a
//! file, a directory tree, or a SQLite database to a callback.

use std::fs;
use std::path::Path;

use verbalize::token::Span;

/// Whitespace-delimited words with their byte ranges.
pub(crate) fn word_spans(text: &str) -> Vec<(usize, usize, &str)> {
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

/// Whether the sorted, disjoint `spans` cover `[start, end)`. `cursor` is
/// carried across calls: words and spans are both in position order, so the
/// whole text costs one pass instead of a scan per word.
pub(crate) fn covered(spans: &[Span], cursor: &mut usize, start: usize, end: usize) -> bool {
    while *cursor < spans.len() && spans[*cursor].range.end <= start {
        *cursor += 1;
    }
    spans
        .get(*cursor)
        .is_some_and(|span| span.range.start < end)
}

/// Every text of `path` — a file, a directory tree, or `sqlite:<path>` —
/// in turn. Non-UTF-8/binary files are silently skipped: this is a text
/// corpus tool.
///
/// A directory tree is walked without following symlinked directories, so a
/// link back to an ancestor cannot repeat files; symlinked files are read.
/// `path` itself may be a symlink.
pub(crate) fn for_each_text(path: &str, f: &mut dyn FnMut(&str)) -> Result<(), String> {
    if let Some(db_path) = path.strip_prefix("sqlite:") {
        each_sqlite(db_path, f)
    } else {
        let p = Path::new(path);
        if p.is_dir() {
            each_dir(p, f)
        } else {
            let text = fs::read_to_string(p).map_err(|e| format!("reading {path}: {e}"))?;
            f(&text);
            Ok(())
        }
    }
}

fn each_dir(dir: &Path, f: &mut dyn FnMut(&str)) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading {}: {e}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        // `file_type` does not resolve links: a link to a directory is not a
        // directory here, and reading it as a file fails and is skipped.
        if file_type.is_dir() {
            each_dir(&path, f)?;
        } else if let Ok(text) = fs::read_to_string(&path) {
            f(&text);
        }
    }
    Ok(())
}

/// Whether a column declared as `declared_type` has SQLite TEXT affinity
/// (https://www.sqlite.org/datatype3.html#determination_of_column_affinity):
/// an `INT` in the name makes it INTEGER before `CHAR`/`CLOB`/`TEXT` is
/// considered.
fn has_text_affinity(declared_type: &str) -> bool {
    let declared = declared_type.to_ascii_uppercase();
    !declared.contains("INT")
        && ["CHAR", "CLOB", "TEXT"]
            .iter()
            .any(|marker| declared.contains(marker))
}

fn each_sqlite(path: &str, f: &mut dyn FnMut(&str)) -> Result<(), String> {
    // Read-only, and without SQLITE_OPEN_CREATE: a mistyped path must fail
    // instead of silently producing an empty database.
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("opening {path}: {e}"))?;

    let mut tables_stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'",
        )
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
            .filter(|(_, ty)| has_text_affinity(ty))
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
            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                // A text-affinity column can still hold a BLOB or NULL, and
                // text that is not UTF-8: none of those are corpus text.
                if let Ok(rusqlite::types::ValueRef::Text(bytes)) = row.get_ref(0) {
                    if let Ok(text) = std::str::from_utf8(bytes) {
                        f(text);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use verbalize::token::{Numeral, Token};

    fn span(start: usize, end: usize) -> Span {
        Span {
            range: start..end,
            token: Token::Cardinal(Numeral::int(1)),
            spoken: String::new(),
            fallback: false,
        }
    }

    #[test]
    fn text_affinity_follows_sqlites_declared_type_rules() {
        for text in [
            "TEXT",
            "VARCHAR(50)",
            "character(20)",
            "NCHAR(55)",
            "CLOB",
            "STRING TEXT",
        ] {
            assert!(has_text_affinity(text), "{text}");
        }
        // `INT` wins over `CHAR`; no `TEXT` marker, or no declared type, is
        // not text.
        for other in [
            "", "INTEGER", "BIGINT", "POINT", "CHARINT", "BLOB", "REAL", "NUMERIC", "DATETIME",
        ] {
            assert!(!has_text_affinity(other), "{other:?}");
        }
    }

    #[test]
    fn covered_advances_over_sorted_disjoint_spans() {
        let spans = vec![span(0, 3), span(10, 12)];
        let mut cursor = 0;
        assert!(covered(&spans, &mut cursor, 0, 3));
        assert!(covered(&spans, &mut cursor, 2, 5));
        assert!(!covered(&spans, &mut cursor, 4, 8));
        assert!(covered(&spans, &mut cursor, 10, 12));
        assert!(!covered(&spans, &mut cursor, 20, 22));
    }
}
