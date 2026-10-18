//! `verbalize normalize|annotate|survey|audit`.

mod audit;
mod corpus;
mod survey;

use std::borrow::Cow;
use std::fmt;
use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use verbalize::{Language, Normalizer, Options, Region};

#[derive(Parser)]
#[command(name = "verbalize", about = "Text normalisation for speech synthesis")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the normalized (spoken-form) text.
    Normalize(CommonArgs),
    /// Print one classified span per line (or a JSON array with --json).
    ///
    /// Lines are `start-end<TAB>class<TAB>original<TAB>spoken<TAB>fallback`.
    /// Inside the original and spoken fields a backslash, tab, newline and
    /// carriage return are written as `\\`, `\t`, `\n` and `\r`, so every span
    /// stays on one line with exactly five fields.
    Annotate(AnnotateArgs),
    /// Scan a corpus for digit/symbol shapes and report coverage.
    Survey(SurveyArgs),
    /// Scan a corpus for unit symbols guessed as an SI prefix on a shorter
    /// symbol that the corpus also uses as an ordinary word.
    Audit(AuditArgs),
}

#[derive(Args)]
struct CommonArgs {
    /// BCP-47 language tag: de, en, ro.
    #[arg(long, default_value = "en")]
    lang: String,
    /// Regional variant, e.g. de-CH, en-US, en-GB.
    #[arg(long)]
    region: Option<String>,
    /// Disable "z. B." → "zum Beispiel" style abbreviation expansion.
    #[arg(long)]
    no_abbreviations: bool,
    /// TSV file of literal<TAB>spoken pairs (Options::lexicon).
    #[arg(long)]
    lexicon: Option<PathBuf>,
    /// Input file, or "-"/omitted for stdin.
    input: Option<String>,
}

#[derive(Args)]
struct AnnotateArgs {
    #[command(flatten)]
    common: CommonArgs,
    /// Emit a JSON array instead of tab-separated lines.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct SurveyArgs {
    /// BCP-47 language tag: de, en, ro.
    #[arg(long, default_value = "en")]
    lang: String,
    /// A text file, a directory of text files, or `sqlite:<path>`.
    path: String,
    /// Exit non-zero if any unhandled shape has count >= N.
    #[arg(long)]
    threshold: Option<u64>,
}

#[derive(Args)]
struct AuditArgs {
    /// BCP-47 language tag: de, en, ro.
    #[arg(long, default_value = "en")]
    lang: String,
    /// A text file, a directory of text files, or `sqlite:<path>`.
    path: String,
    /// Exit non-zero if any suspect token was composed at least N times.
    #[arg(long)]
    threshold: Option<u64>,
}

/// Why a command stopped. Output errors are kept apart because a closed
/// stdout (`| head`) is the reader's choice, not a failure; they carry the
/// exit code the run had earned, so a closed pipe never hides a failed gate.
enum Failure {
    Message(String),
    Output(io::Error, ExitCode),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure::Message(message)
    }
}

impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Self {
        Failure::Output(e, ExitCode::SUCCESS)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Message(message) => f.write_str(message),
            Failure::Output(e, _) => write!(f, "writing output: {e}"),
        }
    }
}

type Outcome = Result<ExitCode, Failure>;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = match cli.command {
        Command::Normalize(args) => run_normalize(args, &mut out),
        Command::Annotate(args) => run_annotate(args, &mut out),
        Command::Survey(args) => run_survey(args, &mut out),
        Command::Audit(args) => run_audit(args, &mut out),
    }
    // A `BufWriter` drops its flush error, so flush where it can be reported.
    .and_then(|code| {
        out.flush()
            .map(|()| code)
            .map_err(|e| Failure::Output(e, code))
    });
    match result {
        Ok(code) => code,
        Err(Failure::Output(e, code)) if e.kind() == io::ErrorKind::BrokenPipe => code,
        Err(e) => {
            eprintln!("verbalize: {e}");
            ExitCode::FAILURE
        }
    }
}

fn read_input(input: &Option<String>) -> Result<String, String> {
    match input.as_deref() {
        None | Some("-") => {
            let mut s = String::new();
            io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| format!("reading stdin: {e}"))?;
            Ok(s)
        }
        Some(path) => fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}")),
    }
}

/// The region named by `s` and the language it belongs to.
fn parse_region(s: &str) -> Result<(Language, Region), String> {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "de-de" | "dede" => Ok((Language::De, Region::DeDe)),
        "de-at" | "deat" => Ok((Language::De, Region::DeAt)),
        "de-ch" | "dech" => Ok((Language::De, Region::DeCh)),
        "en-us" | "enus" => Ok((Language::En, Region::EnUs)),
        "en-gb" | "engb" => Ok((Language::En, Region::EnGb)),
        "ro-ro" | "roro" => Ok((Language::Ro, Region::RoRo)),
        other => Err(format!("unknown region: {other:?}")),
    }
}

fn read_lexicon(path: &PathBuf) -> Result<Vec<(String, String)>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("reading lexicon {}: {e}", path.display()))?;
    let mut entries = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let literal = parts.next().unwrap_or_default();
        let spoken = parts
            .next()
            .ok_or_else(|| format!("{}:{}: expected literal<TAB>spoken", path.display(), n + 1))?;
        entries.push((literal.to_string(), spoken.to_string()));
    }
    Ok(entries)
}

fn build_options(common: &CommonArgs, language: Language) -> Result<Options, String> {
    let region = match common.region.as_deref() {
        Some(tag) => {
            let (region_language, region) = parse_region(tag)?;
            if region_language != language {
                return Err(format!(
                    "--region {tag} is not a region of language {}",
                    language.code()
                ));
            }
            Some(region)
        }
        None => None,
    };
    let lexicon = match &common.lexicon {
        Some(path) => read_lexicon(path)?,
        None => Vec::new(),
    };
    Ok(Options {
        expand_abbreviations: !common.no_abbreviations,
        region,
        lexicon,
    })
}

fn build_normalizer(common: &CommonArgs) -> Result<Normalizer, String> {
    let language: Language = common
        .lang
        .parse()
        .map_err(|e: verbalize::UnsupportedLanguage| e.to_string())?;
    let options = build_options(common, language)?;
    Ok(Normalizer::with_options(language, options))
}

/// Escapes the characters that would break the one-record-per-line,
/// tab-separated layout of `annotate`.
fn tsv_field(field: &str) -> Cow<'_, str> {
    if !field.contains(['\\', '\t', '\n', '\r']) {
        return Cow::Borrowed(field);
    }
    let mut escaped = String::with_capacity(field.len() + 8);
    for c in field.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            c => escaped.push(c),
        }
    }
    Cow::Owned(escaped)
}

fn run_normalize(args: CommonArgs, out: &mut impl Write) -> Outcome {
    let text = read_input(&args.input)?;
    let normalizer = build_normalizer(&args)?;
    write!(out, "{}", normalizer.normalize(&text))?;
    Ok(ExitCode::SUCCESS)
}

fn run_annotate(args: AnnotateArgs, out: &mut impl Write) -> Outcome {
    let text = read_input(&args.common.input)?;
    let normalizer = build_normalizer(&args.common)?;
    let spans = normalizer.annotate(&text);

    if args.json {
        let json: Vec<_> = spans
            .iter()
            .map(|span| {
                serde_json::json!({
                    "byte_start": span.range.start,
                    "byte_end": span.range.end,
                    "class": span.token.class(),
                    "original": &text[span.range.clone()],
                    "spoken": span.spoken,
                    "fallback": span.fallback,
                })
            })
            .collect();
        serde_json::to_writer(&mut *out, &json).map_err(io::Error::from)?;
        writeln!(out)?;
    } else {
        for span in &spans {
            writeln!(
                out,
                "{}-{}\t{}\t{}\t{}\t{}",
                span.range.start,
                span.range.end,
                span.token.class(),
                tsv_field(&text[span.range.clone()]),
                tsv_field(&span.spoken),
                span.fallback,
            )?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_survey(args: SurveyArgs, out: &mut impl Write) -> Outcome {
    let language: Language = args
        .lang
        .parse()
        .map_err(|e: verbalize::UnsupportedLanguage| e.to_string())?;
    let report = survey::run(&args.path, language)?;
    let code = match args.threshold {
        Some(threshold)
            if report
                .unhandled
                .iter()
                .any(|s| s.unhandled as u64 >= threshold) =>
        {
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    };
    survey::print_report(&report, out).map_err(|e| Failure::Output(e, code))?;
    Ok(code)
}

fn run_audit(args: AuditArgs, out: &mut impl Write) -> Outcome {
    let language: Language = args
        .lang
        .parse()
        .map_err(|e: verbalize::UnsupportedLanguage| e.to_string())?;
    let report = audit::run(&args.path, language)?;
    let code = match args.threshold {
        Some(threshold) if report.suspicious().any(|e| e.composed as u64 >= threshold) => {
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    };
    audit::print_report(&report, out).map_err(|e| Failure::Output(e, code))?;
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_field_escapes_separators_and_the_escape_character() {
        assert_eq!(tsv_field("plain"), "plain");
        assert_eq!(tsv_field("a\tb\nc\rd\\e"), "a\\tb\\nc\\rd\\\\e");
    }
}
