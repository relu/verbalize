//! Golden fixtures (design §11.1) and property tests (§11.6).
//!
//! `tests/fixtures/<lang>/<class>.tsv`: `input<TAB>expected`, `#` comments.
//! A `# region: EnGb` directive line switches `Options::region` for the
//! lines that follow (`# region: default` switches back). One test per
//! language; every failing line is reported with input, expected and actual.

use std::fs;
use std::path::{Path, PathBuf};

use verbalize::{Language, Normalizer, Options, Region};

fn fixture_dir(language: Language) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(language.code())
}

fn fixture_files(language: Language) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(fixture_dir(language))
        .expect("fixture directory")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "tsv"))
        .collect();
    files.sort();
    files
}

struct Case {
    line: usize,
    region: Option<Region>,
    input: String,
    expected: String,
}

fn region(name: &str) -> Option<Region> {
    Some(match name {
        "default" => return None,
        "DeDe" => Region::DeDe,
        "DeAt" => Region::DeAt,
        "DeCh" => Region::DeCh,
        "EnUs" => Region::EnUs,
        "EnGb" => Region::EnGb,
        "RoRo" => Region::RoRo,
        other => panic!("unknown region directive {other:?}"),
    })
}

fn cases(path: &Path) -> Vec<Case> {
    let text = fs::read_to_string(path).expect("fixture file");
    let mut current = None;
    let mut cases = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if let Some(name) = line.strip_prefix("# region:") {
            current = region(name.trim());
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (input, expected) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("{}:{}: no tab", path.display(), i + 1));
        cases.push(Case {
            line: i + 1,
            region: current,
            input: input.to_string(),
            expected: expected.to_string(),
        });
    }
    cases
}

/// Normalizers by region, built once: construction compiles every regex.
struct Normalizers {
    language: Language,
    built: std::collections::HashMap<Option<Region>, Normalizer>,
}

impl Normalizers {
    fn new(language: Language) -> Self {
        Normalizers {
            language,
            built: std::collections::HashMap::new(),
        }
    }

    fn get(&mut self, region: Option<Region>) -> &Normalizer {
        let language = self.language;
        self.built.entry(region).or_insert_with(|| {
            Normalizer::with_options(
                language,
                Options {
                    region,
                    ..Options::default()
                },
            )
        })
    }
}

fn run_fixtures(language: Language) {
    let mut normalizers = Normalizers::new(language);
    let mut report = String::new();
    let mut total = 0;
    let mut failed = 0;
    for file in fixture_files(language) {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let mut file_failed = 0;
        let cases = cases(&file);
        for case in &cases {
            total += 1;
            let actual = normalizers.get(case.region).normalize(&case.input);
            if actual != case.expected {
                file_failed += 1;
                report.push_str(&format!(
                    "{name}:{}\n  input:    {:?}\n  expected: {:?}\n  actual:   {actual:?}\n",
                    case.line, case.input, case.expected
                ));
            }
        }
        failed += file_failed;
        eprintln!(
            "{}/{name}: {}/{} pass",
            language.code(),
            cases.len() - file_failed,
            cases.len()
        );
    }
    assert!(
        failed == 0,
        "{failed} of {total} {} fixture lines failed:\n{report}",
        language.code()
    );
}

#[test]
fn de_fixtures() {
    run_fixtures(Language::De);
}

#[test]
fn en_fixtures() {
    run_fixtures(Language::En);
}

#[test]
fn ro_fixtures() {
    run_fixtures(Language::Ro);
}

/// The output contains no ASCII digit, and a second pass is a no-op (§9).
fn check_properties(normalizer: &Normalizer, input: &str) -> Result<(), String> {
    let once = normalizer.normalize(input);
    if once.bytes().any(|b| b.is_ascii_digit()) {
        return Err(format!("digits left: {input:?} -> {once:?}"));
    }
    let twice = normalizer.normalize(&once);
    if twice != once {
        return Err(format!(
            "not idempotent: {input:?} -> {once:?} -> {twice:?}"
        ));
    }
    Ok(())
}

fn properties_over_fixture_inputs(language: Language) {
    let mut normalizers = Normalizers::new(language);
    let failures: Vec<String> = fixture_files(language)
        .iter()
        .flat_map(|f| cases(f))
        .filter_map(|case| check_properties(normalizers.get(case.region), &case.input).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn de_properties_over_fixture_inputs() {
    properties_over_fixture_inputs(Language::De);
}

#[test]
fn en_properties_over_fixture_inputs() {
    properties_over_fixture_inputs(Language::En);
}

#[test]
fn ro_properties_over_fixture_inputs() {
    properties_over_fixture_inputs(Language::Ro);
}

/// xorshift64*: deterministic random digit-rich strings without a dev-dependency.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const COMMON_PIECES: &[&str] = &[
    "0", "1", "2", "5", "9", "10", "19", "30", "99", "100", "1990", "2024", "12345", "007", " ",
    " ", " ", ".", ",", ":", "/", "-", "–", "+", "%", "°", "°C", "€", "$", "mg", "km", "m²", "x",
    "×", "e", "^", "½", "§", "II", "(", ")", "\n", "a", "B", "Ω", "µ", "⁻¹", "²", "₂", "H", "O",
    "IE", "'", "″", "N", "±", "√", "≥", "=", "1,5", "3,14", "0,5", "1.5", "3.14", "0.5", "mmHg",
    "min", "l", "L",
];

fn properties_over_random_strings(language: Language, pieces: &[&str]) {
    let mut all: Vec<&str> = COMMON_PIECES.to_vec();
    all.extend_from_slice(pieces);
    let normalizer = Normalizer::new(language);
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    let mut failures = Vec::new();
    for _ in 0..2000 {
        let length = 1 + (rng.next() % 12) as usize;
        let input: String = (0..length).map(|_| rng.pick(&all)).collect();
        if let Err(e) = check_properties(&normalizer, &input) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures, first 20:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn de_properties_over_random_digit_strings() {
    properties_over_random_strings(
        Language::De,
        &[
            "Uhr", "am", "der", "Klasse", "Platz", "Liter", "Monat", "November", "Tel.", "z. B.",
            "RR", "Kalium", "Titer", "Std.",
        ],
    );
}

#[test]
fn en_properties_over_random_digit_strings() {
    properties_over_random_strings(
        Language::En,
        &[
            "pm", "am", "a.m.", "the", "Grade", "November", "Nov.", "St.", "e.g.", "BP", "titer",
            "hrs", "st", "nd", "th", "1st", "s",
        ],
    );
}

#[test]
fn ro_properties_over_random_digit_strings() {
    properties_over_random_strings(
        Language::Ro,
        &[
            "ora",
            "de",
            "lei",
            "ore",
            "ani",
            "noiembrie",
            "ian.",
            "str.",
            "nr.",
            "TA",
            "titru",
            "al",
            "a",
            "-lea",
            "-a",
            "II-lea",
            "x",
            "mp",
        ],
    );
}

#[test]
fn lexicon_replaces_untouched_shorthand_only() {
    let options = Options {
        lexicon: vec![
            ("RR".into(), "Blutdruck".into()),
            ("SpO2".into(), "Sauerstoffsättigung".into()),
            ("HF".into(), "Herzfrequenz".into()),
            ("AF".into(), "Atemfrequenz".into()),
            ("BMI".into(), "B M I".into()),
            ("GFR".into(), "G F R".into()),
        ],
        ..Options::default()
    };
    let normalizer = Normalizer::with_options(Language::De, options);
    assert_eq!(
        normalizer.normalize("RR 120/80 mmHg"),
        "Blutdruck einhundertzwanzig zu achtzig Millimeter Quecksilbersäule"
    );
    assert_eq!(
        normalizer.normalize("SpO2 92 %, HF 110/min, AF 24/min."),
        "Sauerstoffsättigung zweiundneunzig Prozent, Herzfrequenz einhundertzehn pro Minute, Atemfrequenz vierundzwanzig pro Minute."
    );
    assert_eq!(
        normalizer.normalize("BMI 31 kg/m²"),
        "B M I einunddreißig Kilogramm pro Quadratmeter"
    );
    // Word-bounded and case-sensitive: no replacement inside a word or in another case.
    assert_eq!(normalizer.normalize("HFrEF, rr, GFRs"), "HFrEF, rr, GFRs");
    // Never over a digit span: the ratio keeps its own reading.
    assert_eq!(
        normalizer.normalize("RR 120/80"),
        "Blutdruck einhundertzwanzig zu achtzig"
    );
    let without = Normalizer::new(Language::De);
    assert_eq!(
        without.normalize("RR 120/80"),
        "RR einhundertzwanzig zu achtzig"
    );
}

#[test]
fn abbreviations_can_be_left_alone() {
    let options = Options {
        expand_abbreviations: false,
        ..Options::default()
    };
    let de = Normalizer::with_options(Language::De, options.clone());
    assert_eq!(de.normalize("z. B. 5 km"), "z. B. fünf Kilometer");
    let en = Normalizer::with_options(Language::En, options);
    assert_eq!(en.normalize("e.g. 5 km"), "e.g. five kilometers");
}
