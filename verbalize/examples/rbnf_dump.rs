//! Oracle-comparison helper for `tools/icu-diff/icu_diff.py` (design §11.2).
//!
//! Reads `<locale>\t<ruleset>\t<n>` lines from stdin (one request per line)
//! and prints the RBNF spelling of `n` for that locale/ruleset on stdout,
//! one line per request, in the same order. A request the interpreter has
//! no answer for prints the literal sentinel `<NONE>`.
//!
//! requires verbalize::spell::ruleset — this example will not compile until
//! it lands. The exact entry point it needs:
//!
//! ```ignore
//! pub fn verbalize::spell::ruleset(language: Language, ruleset: &str, n: i128) -> Option<String>;
//! ```
//!
//! `ruleset` is the exact CLDR ruleset name (public or `%%` private); `None`
//! for an unknown name. The returned string is already post-processed (soft
//! hyphens stripped, whitespace collapsed and trimmed), same as everywhere
//! else in the crate.

use std::io::{self, BufRead, Write};
use std::str::FromStr;

use verbalize::Language;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line.expect("read stdin line");
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let locale = parts.next().expect("missing locale field");
        let ruleset = parts.next().expect("missing ruleset field");
        let n: i128 = parts
            .next()
            .expect("missing n field")
            .parse()
            .expect("n is not a valid i128");

        let language = Language::from_str(locale).expect("unsupported locale");
        let spelled = verbalize::spell::ruleset(language, ruleset, n);
        match spelled {
            Some(s) => writeln!(out, "{s}").expect("write stdout"),
            None => writeln!(out, "<NONE>").expect("write stdout"),
        }
    }
}
