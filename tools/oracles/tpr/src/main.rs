//! Reads `lang<TAB>input` lines from stdin and prints text-processing-rs's
//! sentence-level TN for each, one line per request.

use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        let (lang, input) = line.split_once('\t').unwrap_or(("en", &line));
        let spoken = text_processing_rs::tn_normalize_sentence_lang(input, lang);
        writeln!(out, "{}", spoken.replace('\n', " ")).expect("stdout");
    }
}
