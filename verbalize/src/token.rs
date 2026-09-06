//! Token model: what the classifier finds and the grammatical features the
//! spoken form must agree with. Payloads are typed, never pre-spelled.

use std::ops::Range;

/// Grammatical case, as CLDR tags unit patterns (`accusative-count-one`).
/// `Nominative` is CLDR's untagged default form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Case {
    #[default]
    Nominative,
    Accusative,
    Dative,
    Genitive,
}

/// Grammatical gender, as CLDR tags units (`"gender": "feminine"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gender {
    Masculine,
    Feminine,
    Neuter,
}

/// Grammatical number of the governing noun.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Number {
    #[default]
    Singular,
    Plural,
}

/// Adjective declension pattern. German ordinals and "ein" take weak
/// endings after a definite determiner ("am ersten", "der erste") and
/// strong endings without one ("erster November", "seit erstem November").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Declension {
    /// No determiner: bare number before a noun or month.
    #[default]
    Strong,
    /// After a definite article or a contracted preposition (`am`, `im`, `den`).
    Weak,
}

/// Where the governing noun stands relative to the number, for languages
/// whose agreement depends on it (Romanian "20 de euro").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NounPosition {
    #[default]
    Absent,
    Before,
    After,
}

/// What the spoken form of a number must agree with. Decided by the
/// classifier from local context and consumed by the verbalizer;
/// `Agreement::default()` is the bare, context-free reading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Agreement {
    pub case: Case,
    /// Gender of the governing noun, when known from a lexicon, unit or
    /// currency table. `None` selects the neutral standalone form ("eins").
    pub gender: Option<Gender>,
    pub number: Number,
    pub declension: Declension,
    pub noun: NounPosition,
}

/// A written number as parsed: sign, integer magnitude, and the decimal
/// digits after the separator, kept verbatim ("3,10" keeps its zero).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Numeral {
    pub negative: bool,
    pub integer: u128,
    pub fraction: String,
}

impl Numeral {
    pub fn int(integer: u128) -> Self {
        Numeral {
            negative: false,
            integer,
            fraction: String::new(),
        }
    }

    pub fn is_integer(&self) -> bool {
        self.fraction.is_empty()
    }
}

macro_rules! numeral_from {
    ($($t:ty),*) => {$(
        impl From<$t> for Numeral {
            fn from(n: $t) -> Self {
                Numeral::int(u128::from(n))
            }
        }
    )*};
}

numeral_from!(u8, u32, u64);

/// A unit of measure as written: a numerator and any number of `/`
/// denominators, each resolved to a CLDR unit, a per-language extra
/// symbol, or a plain word ("€/Monat").
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Unit {
    /// `None` for a bare rate (`/min`, `mol⁻¹`).
    pub numerator: Option<UnitPart>,
    pub denominators: Vec<UnitPart>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnitPart {
    /// A number inside the denominator (`ml/min/1,73 m²`).
    pub factor: Option<Numeral>,
    pub unit: UnitRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnitRef {
    /// CLDR unit id (`length-kilometer`), with an optional SI prefix as a
    /// power of ten when the symbol was composed (`mmol` = -3 + `concentr-mole`).
    Cldr {
        id: &'static str,
        prefix: Option<i8>,
    },
    /// Per-language extra symbol (`IE`, `mval`), by its symbol.
    Extra(&'static str),
    /// A word after `/` that is not a unit symbol, kept as written.
    Word(String),
}

/// An end of a range: `5–10`, `1.–3.`, `2010–2012`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RangeEnd {
    Cardinal(Numeral),
    Ordinal(u64),
    Year(u64),
}

/// A clock reading as written; `minute` is `None` for a bare hour
/// (`10-14 Uhr`, `2 pm`), `second` is set only by the `hh:mm:ss` form,
/// `meridiem` is the written `am`/`pm`. `is_clock` is English-only: it
/// forces an `hh:mm:ss` reading to a clock (hour, minute, "and N seconds")
/// rather than the default counted-noun duration, when a meridiem or a
/// recognised time zone abbreviation follows in the source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Clock {
    pub hour: u8,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub meridiem: Option<Meridiem>,
    pub is_clock: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Meridiem {
    Am,
    Pm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RatioKind {
    /// `1:640`, `1 : 3` — "zu", "to".
    Ratio,
    /// `120/80 mmHg`, `BP 120/80` — "zu", "over".
    BloodPressure,
    /// `100-110/min`: a range with one rate unit — "bis", "to".
    Range,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PowerBase {
    Number(Numeral),
    Variable(char),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChemicalPart {
    /// Element letters as written (`Na`, `SO`).
    Symbol(String),
    /// Subscript count.
    Count(u32),
    /// Superscript charge: `²⁺` is `{ magnitude: Some(2), negative: false }`.
    Charge {
        magnitude: Option<u32>,
        negative: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    Plus,
    Minus,
    Times,
    DividedBy,
    Equals,
    NotEquals,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Approximately,
    PlusMinus,
    SquareRoot,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MathItem {
    Number(Numeral),
    Variable(char),
    /// A written operand kept as is (`UTC` in `UTC+2`).
    Word(String),
    Operator(Operator),
    Infinity,
    Pi,
    Unit(Unit),
    Percent,
}

/// A magnitude word between a number and its currency or noun (`Mio.`,
/// `Milliarden`, `billion`, `mil.`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scale {
    Thousand,
    Million,
    Billion,
    Trillion,
}

/// A classified token. One variant per semiotic class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Token {
    Cardinal(Numeral),
    /// `1990`, `1990er` (`decade`).
    Year {
        year: u64,
        decade: bool,
    },
    Ordinal(u64),
    Decimal(Numeral),
    /// `2.3`, `3.10`, `1.2.7`: a section, version or clause number — digit
    /// groups joined by periods that no date, decimal or grouped cardinal
    /// claimed. Groups are kept as written (`1.01` keeps its zero).
    Dotted(Vec<String>),
    /// An email address, URL, domain, IPv4 address or `@handle`, as
    /// written; read symbol by symbol (`@` "at", `.` "Punkt", `/`
    /// "Schrägstrich"), letters as words, digits one by one.
    Electronic(String),
    /// `currency` is the ISO 4217 code; `per` is the `/Monat` suffix;
    /// `scale` a magnitude word between amount and currency (`2,5 Mrd. €`).
    Money {
        amount: Numeral,
        currency: &'static str,
        per: Option<UnitPart>,
        scale: Option<Scale>,
    },
    Percent {
        value: Numeral,
        permille: bool,
    },
    /// `scale` is the CLDR unit id after the degree sign (`temperature-celsius`),
    /// `None` for a bare `°`.
    Degrees {
        value: Numeral,
        scale: Option<&'static str>,
    },
    /// `hyphenated` for `75-g-oGTT`, where the unit name joins with hyphens.
    Measure {
        value: Numeral,
        unit: Unit,
        hyphenated: bool,
    },
    Time(Clock),
    TimeRange {
        from: Clock,
        to: Clock,
    },
    /// `day` is `None` for the ISO year-month form (`2003-03`).
    Date {
        day: Option<u8>,
        month: u8,
        year: Option<u64>,
    },
    /// `unit` is `Some` only when a unit right after the range was
    /// claimed with it (`5-10 Min.`), read from `to`'s value; `None`
    /// leaves any trailing noun as plain unclaimed text.
    Range {
        from: RangeEnd,
        to: RangeEnd,
        unit: Option<Unit>,
    },
    Score {
        left: u32,
        right: u32,
    },
    /// Digit groups; `international` for a `+` prefix.
    Telephone {
        groups: Vec<String>,
        international: bool,
    },
    /// A run read digit by digit, separators removed.
    Digits(String),
    /// The abbreviation as written, key into the language's table.
    Abbreviation(String),
    /// Separated digit groups of a fixed shape (a 16-digit card number
    /// `4111 1111 1111 1111`, a US SSN `123-45-6789`): digit by digit with a
    /// pause between groups, like Telephone, but not a phone number.
    DigitGroups(Vec<String>),
    /// A number and a magnitude word with no currency (`1,2 Mio. Einwohner`,
    /// `5 thousand`); a following noun stays as ordinary text after the span.
    Scaled {
        amount: Numeral,
        scale: Scale,
        /// `2 Millionen km/h`: the unit after the scale, read in the plural.
        unit: Option<Unit>,
    },
    /// `½`, `3/4`, `1 ½` (`whole`).
    Fraction {
        whole: Option<Numeral>,
        numerator: u32,
        denominator: u32,
    },
    Paragraph {
        plural: bool,
    },
    /// Roman numeral I–IV, or a range `II-III`.
    Roman {
        from: u8,
        to: Option<u8>,
    },
    Scientific {
        mantissa: Numeral,
        exponent: i32,
        unit: Option<Unit>,
    },
    Power {
        base: PowerBase,
        exponent: i32,
    },
    Chemical(Vec<ChemicalPart>),
    Math(Vec<MathItem>),
    /// `1:640`, `120/80 mmHg`, `100-110/min`.
    Ratio {
        left: Numeral,
        right: Numeral,
        unit: Option<Unit>,
        kind: RatioKind,
    },
    Dose {
        groups: Vec<Numeral>,
        unit: Option<Unit>,
    },
    /// `2x`, `2×/Tag`, `3-mal`.
    Repetition {
        count: Numeral,
        per: Option<Unit>,
    },
    Angle {
        degrees: Numeral,
        minutes: Option<Numeral>,
        seconds: Option<Numeral>,
        compass: Option<char>,
    },
    /// A caller-supplied `Options::lexicon` entry, by its written form.
    Lexicon(String),
}

impl Token {
    /// The semiotic class name, i.e. the variant name without its payload
    /// (`Money`, `Date`); the `class` column of `verbalize annotate`.
    pub fn class(&self) -> &'static str {
        match self {
            Token::Cardinal(_) => "Cardinal",
            Token::Year { .. } => "Year",
            Token::Ordinal(_) => "Ordinal",
            Token::Decimal(_) => "Decimal",
            Token::Dotted(_) => "Dotted",
            Token::Electronic(_) => "Electronic",
            Token::Money { .. } => "Money",
            Token::Scaled { .. } => "Scaled",
            Token::Percent { .. } => "Percent",
            Token::Degrees { .. } => "Degrees",
            Token::Measure { .. } => "Measure",
            Token::Time(_) => "Time",
            Token::TimeRange { .. } => "TimeRange",
            Token::Date { .. } => "Date",
            Token::Range { .. } => "Range",
            Token::Score { .. } => "Score",
            Token::Telephone { .. } => "Telephone",
            Token::Digits(_) => "Digits",
            Token::Abbreviation(_) => "Abbreviation",
            Token::DigitGroups(_) => "DigitGroups",
            Token::Fraction { .. } => "Fraction",
            Token::Paragraph { .. } => "Paragraph",
            Token::Roman { .. } => "Roman",
            Token::Scientific { .. } => "Scientific",
            Token::Power { .. } => "Power",
            Token::Chemical(_) => "Chemical",
            Token::Math(_) => "Math",
            Token::Ratio { .. } => "Ratio",
            Token::Dose { .. } => "Dose",
            Token::Repetition { .. } => "Repetition",
            Token::Angle { .. } => "Angle",
            Token::Lexicon(_) => "Lexicon",
        }
    }
}

/// A classified span of the input with its spoken form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// Byte offsets into the input.
    pub range: Range<usize>,
    pub token: Token,
    pub spoken: String,
    /// True when the class was recognised but the spoken form is a
    /// last-resort reading (digit by digit). The survey tool reports these.
    pub fallback: bool,
}
