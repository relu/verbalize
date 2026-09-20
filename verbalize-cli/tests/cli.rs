//! End-to-end tests: spawn the built `verbalize` binary and check its
//! stdout/stderr/exit code, locking in the behaviour documented in
//! `README.md`'s "## CLI" section.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_verbalize")
}

/// Run `verbalize <args>` feeding `stdin`, wait for it to exit, and
/// capture stdout/stderr/status.
fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn verbalize");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait for verbalize")
}

/// Run `verbalize <args>` with no stdin (a positional file/dir argument
/// supplies the input instead).
fn run_no_stdin(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run verbalize")
}

fn stdout_str(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is valid UTF-8")
}

fn stderr_str(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is valid UTF-8")
}

/// A unique path under the OS temp dir: the pid disambiguates concurrent
/// `cargo test` runs, the counter disambiguates tests within this binary.
fn unique_path(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "verbalize_cli_test_{prefix}_{}_{n}",
        std::process::id()
    ))
}

/// A temp file or directory, removed when the test drops it (including on
/// panic, so a failing assertion never leaks fixtures into the temp dir).
struct TempPath {
    path: PathBuf,
    is_dir: bool,
}

impl TempPath {
    fn file(prefix: &str, contents: &str) -> Self {
        let path = unique_path(prefix);
        fs::write(&path, contents).expect("write temp file");
        TempPath {
            path,
            is_dir: false,
        }
    }

    fn dir(prefix: &str) -> Self {
        let path = unique_path(prefix);
        fs::create_dir(&path).expect("create temp dir");
        TempPath { path, is_dir: true }
    }

    fn str(&self) -> &str {
        self.path.to_str().expect("temp path is valid UTF-8")
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if self.is_dir {
            let _ = fs::remove_dir_all(&self.path);
        } else {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// README "## Languages" table, English row: `normalize`, default `--lang`
/// (`en` since 0.1.2).
#[test]
fn normalize_stdin_default_lang_is_english() {
    let output = run(
        &["normalize"],
        "On November 1, 2026 I paid $8.80 at 2:30 pm.",
    );
    assert!(output.status.success());
    assert_eq!(
        stdout_str(&output),
        "On November first, twenty twenty-six I paid eight dollars and eighty cents at two thirty p m."
    );
}

/// README's top-of-file German example, reproduced through `--lang de`.
#[test]
fn normalize_lang_de() {
    let output = run(
        &["normalize", "--lang", "de"],
        "Am 1. November kostet es 8,80 €.",
    );
    assert!(output.status.success());
    assert_eq!(
        stdout_str(&output),
        "Am ersten November kostet es acht Euro achtzig."
    );
}

/// README "## CLI" `annotate` example: one classified span per line,
/// `byte_start-byte_end<TAB>Class<TAB>original<TAB>spoken<TAB>fallback`.
#[test]
fn annotate_plain_de_column_shape() {
    let output = run(
        &["annotate", "--lang", "de"],
        "Am 1. November um 19.30 Uhr kostet es 8,80 €.",
    );
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert_eq!(
        stdout,
        "3-14\tDate\t1. November\tersten November\tfalse\n\
         18-27\tTime\t19.30 Uhr\tneunzehn Uhr dreißig\tfalse\n\
         38-46\tMoney\t8,80 €\tacht Euro achtzig\tfalse\n"
    );

    let first_line = stdout.lines().next().expect("at least one span");
    let columns: Vec<&str> = first_line.split('\t').collect();
    assert_eq!(
        columns.len(),
        5,
        "expected 5 tab-separated columns, got {columns:?}"
    );
    assert_eq!(columns[0], "3-14");
    assert_eq!(columns[1], "Date");
    assert_eq!(columns[2], "1. November");
    assert_eq!(columns[3], "ersten November");
    assert_eq!(columns[4], "false");
}

/// Same input as above, `--json`: an array of objects with the same fields.
#[test]
fn annotate_json_de() {
    let output = run(
        &["annotate", "--json", "--lang", "de"],
        "Am 1. November um 19.30 Uhr kostet es 8,80 €.",
    );
    assert!(output.status.success());
    let spans: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("annotate --json prints one JSON array");
    let spans = spans.as_array().expect("top-level JSON array");
    assert_eq!(spans.len(), 3);

    let money = &spans[2];
    assert_eq!(money["byte_start"], 38);
    assert_eq!(money["byte_end"], 46);
    assert_eq!(money["class"], "Money");
    assert_eq!(money["original"], "8,80 €");
    assert_eq!(money["spoken"], "acht Euro achtzig");
    assert_eq!(money["fallback"], false);
}

/// README `## Options` lexicon example. Reproduced with `--lang de`: the
/// documented output ("Prozent") is German, even though the CLI's default
/// `--lang` is `en`.
#[test]
fn lexicon_file() {
    let lexicon = TempPath::file("lexicon", "SpO2\tSauerstoffsättigung\n");
    let output = run(
        &["normalize", "--lang", "de", "--lexicon", lexicon.str()],
        "SpO2 92 %",
    );
    assert!(output.status.success());
    assert_eq!(
        stdout_str(&output),
        "Sauerstoffsättigung zweiundneunzig Prozent"
    );
}

/// `--region`: `en-US` (default) reads `11/01/2026` month-first, `en-GB`
/// reads it day-first (`verbalize/tests/fixtures/en/date.tsv`).
#[test]
fn region_flag_changes_date_order() {
    let us_default = run(&["normalize", "--lang", "en"], "11/01/2026");
    assert!(us_default.status.success());
    assert_eq!(stdout_str(&us_default), "November first twenty twenty-six");

    let us_explicit = run(
        &["normalize", "--lang", "en", "--region", "en-us"],
        "11/01/2026",
    );
    assert!(us_explicit.status.success());
    assert_eq!(stdout_str(&us_explicit), stdout_str(&us_default));

    let gb = run(
        &["normalize", "--lang", "en", "--region", "en-gb"],
        "11/01/2026",
    );
    assert!(gb.status.success());
    assert_eq!(stdout_str(&gb), "the eleventh of January twenty twenty-six");
}

/// `--no-abbreviations` leaves an abbreviation unexpanded; without it the
/// abbreviation is expanded (default `expand_abbreviations: true`).
#[test]
fn no_abbreviations_flag() {
    let expanded = run(&["normalize", "--lang", "en"], "e.g. 5 km");
    assert!(expanded.status.success());
    assert_eq!(stdout_str(&expanded), "for example five kilometers");

    let left_alone = run(
        &["normalize", "--lang", "en", "--no-abbreviations"],
        "e.g. 5 km",
    );
    assert!(left_alone.status.success());
    assert_eq!(stdout_str(&left_alone), "e.g. five kilometers");
}

/// README: `verbalize survey corpus/ --threshold 5   # non-zero exit if an
/// unhandled shape occurs 5+ times (for CI)`.
#[test]
fn survey_threshold_exit_code() {
    let dir = TempPath::dir("survey");
    // Three occurrences of the same unhandled shape (a long digit run).
    fs::write(
        dir.path.join("a.txt"),
        "Case 007 opened. Case 007 reopened. Case 007 closed.",
    )
    .expect("write corpus file");

    let below_threshold = run_no_stdin(&["survey", dir.str(), "--lang", "en", "--threshold", "4"]);
    assert!(
        below_threshold.status.success(),
        "3 occurrences must not trip a threshold of 4: {}",
        stdout_str(&below_threshold)
    );

    let at_threshold = run_no_stdin(&["survey", dir.str(), "--lang", "en", "--threshold", "3"]);
    assert!(
        !at_threshold.status.success(),
        "3 occurrences must trip a threshold of 3"
    );
    assert_eq!(at_threshold.status.code(), Some(1));
}

/// A smoke-level `audit` invocation: the subcommand runs over a small
/// corpus and exits cleanly. `audit.rs` already unit-tests the composed-unit
/// detection itself.
#[test]
fn audit_smoke() {
    let dir = TempPath::dir("audit");
    fs::write(
        dir.path.join("a.txt"),
        "Der Patient hatte 29 Grad Fieber, der Grad der Erkrankung war hoch.",
    )
    .expect("write corpus file");

    let output = run_no_stdin(&["audit", dir.str(), "--lang", "de"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("composed"));
}

/// A missing input file is a clean CLI error (exit 1, message on stderr),
/// not a panic/backtrace: `read_input` surfaces `fs::read_to_string`'s
/// error through `main`'s `eprintln!("verbalize: {e}")`.
#[test]
fn missing_file_is_a_clean_error() {
    let missing = unique_path("missing");
    let output = run_no_stdin(&["normalize", missing.to_str().expect("utf8 path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout_str(&output).is_empty());
    let stderr = stderr_str(&output);
    assert!(
        stderr.starts_with("verbalize: reading "),
        "stderr: {stderr:?}"
    );
    assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
}

/// Non-UTF-8 input is the same clean-error path, not a panic.
#[test]
fn non_utf8_file_is_a_clean_error() {
    let path = unique_path("non_utf8");
    fs::write(&path, [0xff, 0xfe, 0x00, b'b', b'a', b'd']).expect("write non-utf8 file");
    let output = run_no_stdin(&["normalize", path.to_str().expect("utf8 path")]);
    let _ = fs::remove_file(&path);

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_str(&output);
    assert!(
        stderr.starts_with("verbalize: reading "),
        "stderr: {stderr:?}"
    );
    assert!(stderr.contains("UTF-8"), "stderr: {stderr:?}");
    assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
}
