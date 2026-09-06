//! `verbalize normalize|annotate|survey`.

mod survey;

use std::fs;
use std::io::{self, Read};
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
    Annotate(AnnotateArgs),
    /// Scan a corpus for digit/symbol shapes and report coverage.
    Survey(SurveyArgs),
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
    /// A text file, a directory of text files, or `sqlite:<path>`.
    path: String,
    /// Exit non-zero if any unhandled shape has count >= N.
    #[arg(long)]
    threshold: Option<u64>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Normalize(args) => run_normalize(args),
        Command::Annotate(args) => run_annotate(args),
        Command::Survey(args) => run_survey(args),
    };
    match result {
        Ok(code) => code,
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

fn parse_region(s: &str) -> Result<Region, String> {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "de-de" | "dede" => Ok(Region::DeDe),
        "de-at" | "deat" => Ok(Region::DeAt),
        "de-ch" | "dech" => Ok(Region::DeCh),
        "en-us" | "enus" => Ok(Region::EnUs),
        "en-gb" | "engb" => Ok(Region::EnGb),
        "ro-ro" | "roro" => Ok(Region::RoRo),
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

fn build_options(common: &CommonArgs) -> Result<Options, String> {
    let region = common.region.as_deref().map(parse_region).transpose()?;
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
    let options = build_options(common)?;
    Ok(Normalizer::with_options(language, options))
}

fn run_normalize(args: CommonArgs) -> Result<ExitCode, String> {
    let text = read_input(&args.input)?;
    let normalizer = build_normalizer(&args)?;
    print!("{}", normalizer.normalize(&text));
    Ok(ExitCode::SUCCESS)
}

fn run_annotate(args: AnnotateArgs) -> Result<ExitCode, String> {
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
        println!(
            "{}",
            serde_json::to_string(&json).map_err(|e| e.to_string())?
        );
    } else {
        for span in &spans {
            println!(
                "{}-{}\t{}\t{}\t{}\t{}",
                span.range.start,
                span.range.end,
                span.token.class(),
                &text[span.range.clone()],
                span.spoken,
                span.fallback,
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_survey(args: SurveyArgs) -> Result<ExitCode, String> {
    let report = survey::run(&args.path)?;
    survey::print_report(&report);
    if let Some(threshold) = args.threshold {
        if report.unhandled.iter().any(|s| s.count as u64 >= threshold) {
            return Ok(ExitCode::FAILURE);
        }
    }
    Ok(ExitCode::SUCCESS)
}
