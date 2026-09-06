//! Text normalisation for speech synthesis.
//!
//! Turns text written for the eye ("8,80 €", "19.30 Uhr", "1. November")
//! into text written for the ear ("acht Euro achtzig", "neunzehn Uhr
//! dreißig", "erster November"), driven by vendored CLDR data.
//!
//! ```
//! use verbalize::{Language, Normalizer};
//!
//! let de = Normalizer::new(Language::De);
//! assert_eq!(de.normalize("Am 1. November kostet es 8,80 €."),
//!            "Am ersten November kostet es acht Euro achtzig.");
//! ```

#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

pub mod data;
mod lang;
mod pipeline;
mod rbnf;
pub mod spell;
pub mod token;

use token::Span;

/// Turns written text into its spoken form for one language. Construction
/// compiles the language's recognisers once; keep one per language for the
/// process lifetime (it is `Send + Sync`).
pub struct Normalizer {
    lang: Box<dyn lang::Lang>,
    options: Options,
}

impl Normalizer {
    pub fn new(language: Language) -> Self {
        Normalizer::with_options(language, Options::default())
    }

    /// English and Romanian have no language module yet: their normalizer
    /// returns the input verbatim.
    pub fn with_options(language: Language, options: Options) -> Self {
        let lang = lang::new(language, &options);
        Normalizer { lang, options }
    }

    /// Spoken form of `text`. Never fails; unrecognised input is returned
    /// verbatim. Idempotent: `normalize(normalize(t)) == normalize(t)`.
    pub fn normalize(&self, text: &str) -> String {
        pipeline::render(text, &self.annotate(text))
    }

    /// The classified spans with their spoken forms, for tooling, tests,
    /// and future SSML rendering. Sorted by position, non-overlapping.
    pub fn annotate(&self, text: &str) -> Vec<Span> {
        pipeline::annotate(self.lang.as_ref(), &self.options, text)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
}

/// Reading options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Expand "z. B." → "zum Beispiel" etc. Default true.
    pub expand_abbreviations: bool,
    /// Regional variant where it changes the reading. Default: the
    /// language's primary region.
    pub region: Option<Region>,
    /// Literal, case-sensitive, word-bounded shorthand → spoken-word pairs,
    /// a caller-supplied domain lexicon such as `[("RR", "Blutdruck")]` for
    /// clinical prose. Applied after every semiotic class, so an entry can
    /// never shadow a phone number, dose or ratio; an entry may itself
    /// contain a digit (`SpO2`), in which case it wins over the bare
    /// glued-digit reading. Default empty.
    pub lexicon: Vec<(String, String)>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            expand_abbreviations: true,
            region: None,
            lexicon: Vec::new(),
        }
    }
}

/// Regional variants that change a reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Region {
    DeDe,
    DeAt,
    DeCh,
    EnUs,
    EnGb,
    RoRo,
}

/// A supported language. Parsed from a BCP-47 tag with [`FromStr`]; the
/// region and other subtags are accepted and ignored here (`Options::region`
/// selects regional readings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    De,
    En,
    Ro,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::De, Language::En, Language::Ro];

    /// BCP-47 language subtag.
    pub fn code(self) -> &'static str {
        self.data().language
    }

    pub(crate) fn data(self) -> &'static data::LangData {
        match self {
            Language::De => &data::de::DE,
            Language::En => &data::en::EN,
            Language::Ro => &data::ro::RO,
        }
    }
}

/// The tag was not valid BCP-47, or its language is not supported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedLanguage(pub String);

impl fmt::Display for UnsupportedLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported language: {:?}", self.0)
    }
}

impl std::error::Error for UnsupportedLanguage {}

impl FromStr for Language {
    type Err = UnsupportedLanguage;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let locale = icu_locale_core::Locale::try_from_str(tag)
            .map_err(|_| UnsupportedLanguage(tag.to_string()))?;
        Language::ALL
            .into_iter()
            .find(|l| l.code() == locale.id.language.as_str())
            .ok_or_else(|| UnsupportedLanguage(tag.to_string()))
    }
}

/// Parses RBNF rulesets as `Rbnf::new` would, without the rest of the
/// crate. `cargo xtask gen` calls this on the freshly vendored JSON so that
/// rule syntax outside the interpreter's scope fails data generation, never
/// a reading at runtime. Not part of the public API.
#[doc(hidden)]
pub fn check_rbnf(
    language: &str,
    symbols: [&str; 3],
    rulesets: &[(&str, &[(&str, &str)])],
) -> Result<(), String> {
    let [decimal, group, minus] = symbols;
    let symbols = rbnf::Symbols::new(decimal, group, minus);
    rbnf::Rbnf::compile(language, symbols, rulesets.iter().copied())
        .map(drop)
        .map_err(|e| e.to_string())
}
