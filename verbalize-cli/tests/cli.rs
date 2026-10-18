//! End-to-end tests: spawn the built `verbalize` binary and check its
//! stdout/stderr/exit code, locking in the behaviour documented in
//! `README.md`'s "## CLI" section.

use std::fs;
use std::io::{BufRead, BufReader, Write};
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

/// Spawn `verbalize <args>`, feed it far more input than the output pipe can
/// buffer, read a single line of output, then close the pipe like `| head -1`
/// does. The child must stop quietly: the reader going away is not an error.
fn assert_quiet_exit_when_reader_closes(args: &[&str], line: &'static str) {
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn verbalize");

    let mut stdin = child.stdin.take().expect("child stdin");
    let feeder = std::thread::spawn(move || {
        let chunk = line.repeat(1000);
        for _ in 0..40 {
            // The child may exit before it has read everything.
            if stdin.write_all(chunk.as_bytes()).is_err() {
                break;
            }
        }
    });

    let mut first = String::new();
    let mut stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    stdout.read_line(&mut first).expect("read first line");
    assert!(!first.is_empty(), "no output before the pipe closed");
    drop(stdout);

    let output = child.wait_with_output().expect("wait for verbalize");
    feeder.join().expect("feeder thread");
    let stderr = stderr_str(&output);
    assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
    assert!(stderr.is_empty(), "stderr: {stderr:?}");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn normalize_exits_quietly_when_stdout_closes() {
    assert_quiet_exit_when_reader_closes(&["normalize", "--lang", "de"], "5 kg\n");
}

#[test]
fn annotate_exits_quietly_when_stdout_closes() {
    assert_quiet_exit_when_reader_closes(&["annotate", "--lang", "de"], "5 kg\n");
}

/// `sqlite:` input is read-only: a mistyped path is an error, and no empty
/// database appears at that path (which would also make a CI gate pass).
#[test]
fn missing_sqlite_database_is_an_error_and_is_not_created() {
    let missing = unique_path("missing_db");
    let spec = format!("sqlite:{}", missing.to_str().expect("utf8 path"));
    for command in ["survey", "audit"] {
        let output = run_no_stdin(&[command, &spec, "--threshold", "1"]);
        assert_eq!(output.status.code(), Some(1), "{command}");
        assert!(stdout_str(&output).is_empty(), "{command}");
        let stderr = stderr_str(&output);
        assert!(stderr.starts_with("verbalize: opening "), "{stderr:?}");
        assert!(stderr.contains("unable to open"), "{stderr:?}");
        assert!(!missing.exists(), "{command} created {}", missing.display());
    }
}

/// Columns are scanned by SQLite's type affinity, so `VARCHAR(50)` counts as
/// text; a BLOB or NULL in a text column is skipped, not fatal; and a user
/// table whose name merely resembles `sqlite_*` is still scanned.
#[test]
fn sqlite_scans_every_text_affinity_column_and_skips_other_values() {
    let db = TempPath::file("db", "");
    {
        let conn = rusqlite::Connection::open(&db.path).expect("create db");
        conn.execute_batch(
            "CREATE TABLE notes (id INTEGER, body VARCHAR(50), raw TEXT, n INT);
             INSERT INTO notes VALUES (1, 'Case 007 opened.', NULL, 5);
             INSERT INTO notes VALUES (2, 'Case 007 closed.', x'ff00fe', 6);
             CREATE TABLE sqlitex (t TEXT);
             INSERT INTO sqlitex VALUES ('Case 007 reopened.');",
        )
        .expect("populate db");
    }

    let spec = format!("sqlite:{}", db.str());
    let output = run_no_stdin(&["survey", &spec, "--lang", "en"]);
    assert!(output.status.success(), "stderr: {}", stderr_str(&output));
    let report = stdout_str(&output);
    // The shape row of "007": two VARCHAR rows plus the `sqlitex` row.
    let row = report
        .lines()
        .find(|l| l.split('\t').nth(2) == Some("007"))
        .unwrap_or_else(|| panic!("no 007 shape row in {report:?}"));
    assert_eq!(row.split('\t').nth(1), Some("3"), "{report:?}");
}

/// A directory link pointing at an ancestor must not multiply the files
/// beneath it; a root given explicitly is followed even if it is a link.
#[cfg(unix)]
#[test]
fn symlinked_directories_are_not_followed() {
    let dir = TempPath::dir("loop");
    let sub = dir.path.join("sub");
    fs::create_dir(&sub).expect("create sub");
    fs::write(sub.join("a.txt"), "Case 007 opened.").expect("write corpus file");
    std::os::unix::fs::symlink("..", sub.join("up")).expect("create loop link");

    let output = run_no_stdin(&["survey", dir.str(), "--lang", "en"]);
    assert!(output.status.success());
    let report = stdout_str(&output);
    let row = report
        .lines()
        .find(|l| l.split('\t').nth(2) == Some("007"))
        .unwrap_or_else(|| panic!("no 007 shape row in {report:?}"));
    assert_eq!(row.split('\t').nth(1), Some("1"), "{report:?}");

    let link_root = unique_path("loop_root");
    std::os::unix::fs::symlink(&dir.path, &link_root).expect("create root link");
    let output = run_no_stdin(&[
        "survey",
        link_root.to_str().expect("utf8 path"),
        "--lang",
        "en",
    ]);
    let _ = fs::remove_file(&link_root);
    assert!(
        stdout_str(&output).contains("007"),
        "root link not followed"
    );
}

/// Symlinked files hold no loop risk and are read like regular files.
#[cfg(unix)]
#[test]
fn symlinked_files_are_read() {
    let dir = TempPath::dir("filelink");
    let outside = TempPath::file("filelink_target", "Case 007 opened.");
    std::os::unix::fs::symlink(&outside.path, dir.path.join("link.txt")).expect("link");

    let output = run_no_stdin(&["survey", dir.str(), "--lang", "en"]);
    assert!(stdout_str(&output).contains("007"));
}

/// `annotate` TSV stays one record per line with five fields, however odd
/// the span text is: `\`, tab, newline and CR are written as `\\`, `\t`,
/// `\n`, `\r`.
#[test]
fn annotate_tsv_escapes_tabs_and_backslashes() {
    let lexicon = TempPath::file("lexicon_escape", "foo\ta\tb\\c\n");
    let output = run(
        &["annotate", "--lang", "en", "--lexicon", lexicon.str()],
        "foo\n",
    );
    assert!(output.status.success(), "stderr: {}", stderr_str(&output));
    let stdout = stdout_str(&output);
    let mut lines = stdout.lines();
    let line = lines.next().expect("one record");
    assert!(lines.next().is_none(), "{stdout:?}");
    let fields: Vec<&str> = line.split('\t').collect();
    assert_eq!(fields.len(), 5, "{line:?}");
    assert_eq!(fields[2], "foo");
    assert_eq!(fields[3], "a\\tb\\\\c");
}

/// A `--region` for another language than `--lang` is an error, not a silent
/// no-op; a region of the same language still works, with or without a
/// region subtag on `--lang`.
#[test]
fn region_must_match_language() {
    let mismatch = run(&["normalize", "--lang", "de", "--region", "en-GB"], "5 kg");
    assert_eq!(mismatch.status.code(), Some(1));
    assert!(stdout_str(&mismatch).is_empty());
    let stderr = stderr_str(&mismatch);
    assert!(stderr.starts_with("verbalize: "), "{stderr:?}");
    assert!(stderr.contains("en-GB"), "{stderr:?}");

    for args in [
        ["normalize", "--lang", "de", "--region", "de-CH"],
        ["normalize", "--lang", "de-AT", "--region", "de_ch"],
    ] {
        let ok = run(&args, "5 kg");
        assert!(ok.status.success(), "{args:?}: {}", stderr_str(&ok));
    }
}

/// With `--strict` a missing corpus is a failure: otherwise a release check
/// run before fetching anything would pass having checked nothing. The script
/// is copied into a scratch tree so the repository's own `.cache` is not read.
#[cfg(unix)]
#[test]
fn corpus_check_strict_fails_without_a_corpus() {
    let tree = TempPath::dir("check_sh");
    let here = tree.path.join("tools").join("corpus");
    fs::create_dir_all(&here).expect("create script dir");
    let script = here.join("check.sh");
    fs::write(&script, include_str!("../../tools/corpus/check.sh")).expect("write script");

    let strict = Command::new("bash")
        .arg(&script)
        .arg("--strict")
        .output()
        .expect("run check.sh");
    assert!(!strict.status.success());
    assert!(
        stderr_str(&strict).contains("no corpus"),
        "stderr: {:?}",
        stderr_str(&strict)
    );

    let lenient = Command::new("bash")
        .arg(&script)
        .output()
        .expect("run check.sh");
    assert!(lenient.status.success());
}

/// Any other write failure is reported, with a non-zero exit, not swallowed
/// like a closed pipe. `/dev/full` fails every write with ENOSPC.
#[cfg(target_os = "linux")]
#[test]
fn write_errors_other_than_a_closed_pipe_fail_the_run() {
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let output = Command::new(bin())
        .args(["normalize", "--lang", "en"])
        .stdin(Stdio::piped())
        .stdout(full)
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().expect("stdin").write_all(b"5 kg")?;
            child.wait_with_output()
        })
        .expect("run verbalize");
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_str(&output);
    assert!(
        stderr.starts_with("verbalize: writing output: "),
        "{stderr:?}"
    );
    assert!(!stderr.contains("panicked"), "{stderr:?}");
}
