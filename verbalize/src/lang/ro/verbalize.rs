//! Romanian spoken forms. Numbers come from CLDR ro RBNF; the "de" before
//! a noun from 20 on is decided here from the CLDR plural category
//! (`other` ⟺ "de"); ordinals are derived from the cardinal words
//! because CLDR ro has no ordinal rulesets (`docs/rules/ro.md`).

use icu_plurals::PluralCategory;

use super::lexicon::*;
use super::Romanian;
use crate::lang::common::{chemical_words, dotted_words, electronic_words, signed_int, unit_words};
use crate::lang::Verbalized;
use crate::rbnf::Value;
use crate::spell;
use crate::token::*;
use crate::Language::Ro;

fn value(n: &Numeral) -> Value<'_> {
    Value::decimal(n.negative, n.integer, &n.fraction)
}

/// Cardinal words: the gendered ruleset when the noun's gender is known
/// ("două ore", "douăzeci și două de ore"), with the article forms
/// "un"/"o" for a bare 1 before a noun; `%spellout-numbering` otherwise.
/// From 1000 up the thousands groups are composed here, because CLDR ro's
/// rules omit the "de" before "mii"/"milioane" ("douăzeci și două de mii").
pub(super) fn words(n: &Numeral, gender: Option<Gender>) -> String {
    if n.is_integer() && n.integer == 1 && !n.negative {
        match gender {
            Some(Gender::Feminine) => return "o".to_string(),
            Some(_) => return "un".to_string(),
            None => {}
        }
    }
    if n.integer >= 1_000_000_000_000_000_000 {
        let mut digits = spell::digits(Ro, &n.integer.to_string());
        if !n.fraction.is_empty() {
            digits.push_str(" virgulă ");
            digits.push_str(&spell::digits(Ro, &n.fraction));
        }
        return if n.negative {
            format!("minus {digits}")
        } else {
            digits
        };
    }
    let mut out = String::new();
    if n.negative {
        out.push_str("minus ");
    }
    out.push_str(&integer_words(n.integer, gender));
    if !n.fraction.is_empty() {
        out.push_str(" virgulă ");
        let fraction: Vec<String> = n
            .fraction
            .chars()
            .map(|d| ruleset_words(u128::from(d.to_digit(10).unwrap_or(0)), gender))
            .collect();
        out.push_str(&fraction.join(" "));
    }
    out
}

fn ruleset_words(n: u128, gender: Option<Gender>) -> String {
    let ruleset = match gender {
        Some(Gender::Masculine) => "%spellout-cardinal-masculine",
        Some(Gender::Feminine) => "%spellout-cardinal-feminine",
        Some(Gender::Neuter) => "%spellout-cardinal-neuter",
        None => "%spellout-numbering",
    };
    standard(spell::spell(Ro, ruleset, Value::int(n as i128)))
}

/// Thousands groups: `(scale, singular, plural, gender of the noun)`.
const SCALES: &[(u128, &str, &str, Gender)] = &[
    (
        1_000_000_000_000_000,
        "cvadrilion",
        "cvadrilioane",
        Gender::Neuter,
    ),
    (1_000_000_000_000, "trilion", "trilioane", Gender::Neuter),
    (1_000_000_000, "miliard", "miliarde", Gender::Neuter),
    (1_000_000, "milion", "milioane", Gender::Neuter),
    (1_000, "mie", "mii", Gender::Feminine),
];

fn integer_words(n: u128, gender: Option<Gender>) -> String {
    if n < 1000 {
        return ruleset_words(n, gender);
    }
    let mut parts = Vec::new();
    let mut rest = n;
    for (scale, singular, plural, scale_gender) in SCALES {
        let count = rest / scale;
        rest %= scale;
        if count == 0 {
            continue;
        }
        let noun = if count == 1 { singular } else { plural };
        let count_words = words(&Numeral::int(count), Some(*scale_gender));
        let link = if plural_category(count) == PluralCategory::Other {
            " de"
        } else {
            ""
        };
        parts.push(format!("{count_words}{link} {noun}"));
    }
    if rest > 0 {
        parts.push(ruleset_words(rest, gender));
    }
    parts.join(" ")
}

/// CLDR ro plural category of an integer, for the "de" decision.
fn plural_category(n: u128) -> PluralCategory {
    let n = u64::try_from(n).unwrap_or(u64::MAX);
    RULES.category_for(icu_plurals::PluralOperands::from(n))
}

static RULES: std::sync::LazyLock<icu_plurals::PluralRules> = std::sync::LazyLock::new(|| {
    icu_plurals::PluralRules::try_new(icu_locale_core::locale!("ro").into(), Default::default())
        .expect("compiled plural data")
});

fn year_words(year: u64) -> String {
    words(&Numeral::from(year), None)
}

/// DOOM3's standard forms over CLDR's variants ("paisprezece", "o sută").
fn standard(mut words: String) -> String {
    for (variant, standard) in STANDARD_FORMS {
        if words.contains(variant) {
            words = words.replace(variant, standard);
        }
    }
    words
}

/// `al doilea`, `a doua`, `al treilea`, `a treia`, `al douăzeci și unulea`,
/// `a douăzeci și una`: the article plus the cardinal with the ordinal
/// ending on its last word (Gramatica Academiei II §3.3.2).
pub(super) fn ordinal_words(n: u64, gender: Gender) -> String {
    let feminine = gender == Gender::Feminine;
    if n == 1 {
        return if feminine { "prima" } else { "primul" }.to_string();
    }
    let cardinal = words(
        &Numeral::int(u128::from(n)),
        Some(if feminine {
            Gender::Feminine
        } else {
            Gender::Masculine
        }),
    );
    let (head, last) = cardinal.rsplit_once(' ').unwrap_or(("", &cardinal));
    let last = if feminine {
        feminine_ordinal(last)
    } else {
        masculine_ordinal(last)
    };
    let article = if feminine { "a" } else { "al" };
    if head.is_empty() {
        format!("{article} {last}")
    } else {
        format!("{article} {head} {last}")
    }
}

fn masculine_ordinal(word: &str) -> String {
    match word {
        "unu" => "unulea".into(),
        "sută" => "sutălea".into(),
        "mie" => "miilea".into(),
        "milion" => "milionulea".into(),
        "miliard" => "miliardulea".into(),
        w if w.ends_with(['i', 'u', 'e', 'ă']) => format!("{w}lea"),
        w => format!("{w}ulea"),
    }
}

fn feminine_ordinal(word: &str) -> String {
    match word {
        "una" => "una".into(),
        "două" => "doua".into(),
        "trei" => "treia".into(),
        "patru" => "patra".into(),
        "nouă" => "noua".into(),
        "sută" => "suta".into(),
        "mie" => "mia".into(),
        "milion" => "milioana".into(),
        "miliard" => "miliarda".into(),
        w if w.ends_with('i') => format!("{}ea", &w[..w.len() - 1]),
        w if w.ends_with('e') => format!("{w}a"),
        w if w.ends_with('ă') => format!("{}a", &w[..w.len() - 'ă'.len_utf8()]),
        w => format!("{w}a"),
    }
}

impl Romanian {
    fn category(&self, n: &Numeral) -> PluralCategory {
        self.units.category(n)
    }

    /// The number before a noun: "de" from 20 on (CLDR plural `other`).
    fn count_words(&self, n: &Numeral, gender: Option<Gender>) -> String {
        let mut out = words(n, gender);
        if self.category(n) == PluralCategory::Other {
            out.push_str(" de");
        }
        out
    }

    /// The hour is feminine ("ora două", "douăsprezece") except 1 and 21.
    fn hour_words(&self, hour: u8) -> String {
        let gender = if hour % 20 == 1 {
            Gender::Masculine
        } else {
            Gender::Feminine
        };
        let n = Numeral::from(hour);
        standard(spell::spell(
            Ro,
            if gender == Gender::Feminine {
                "%spellout-cardinal-feminine"
            } else {
                "%spellout-numbering"
            },
            value(&n),
        ))
    }

    fn clock_words(&self, clock: Clock) -> String {
        match clock.minute {
            None | Some(0) => format!("ora {}", self.hour_words(clock.hour)),
            Some(m) => format!(
                "{} și {}",
                self.hour_words(clock.hour),
                words(&m.into(), None)
            ),
        }
    }

    /// First word of the CLDR display name: "leu"/"lei", "dolar"/"dolari".
    fn head_noun(&self, code: &str, n: &Numeral) -> String {
        let cldr = self.data.currencies.iter().find(|c| c.code == code);
        let category = self.category(n);
        let name = cldr
            .and_then(|c| {
                c.names
                    .iter()
                    .find(|(p, _)| *p == category)
                    .map(|(_, n)| *n)
            })
            .or_else(|| {
                cldr.and_then(|c| {
                    c.names
                        .iter()
                        .find(|(p, _)| *p == PluralCategory::Other)
                        .map(|(_, n)| *n)
                })
            })
            .or(cldr.map(|c| c.name))
            .unwrap_or(code);
        name.split(' ').next().unwrap_or(name).to_string()
    }

    /// `3 mil.` → "trei milioane", `1 mld.` → "un miliard", `20 mii` →
    /// "douăzeci de mii"; with `de` when a noun follows ("… de locuitori").
    fn scaled_words(&self, amount: &Numeral, scale: Scale, noun_follows: bool) -> String {
        let one = amount.is_integer() && amount.integer == 1 && !amount.negative;
        let (noun, gender) = match scale {
            Scale::Thousand if one => ("mie", Gender::Feminine),
            Scale::Thousand => ("mii", Gender::Feminine),
            Scale::Million if one => ("milion", Gender::Neuter),
            Scale::Million => ("milioane", Gender::Neuter),
            Scale::Billion if one => ("miliard", Gender::Neuter),
            Scale::Billion => ("miliarde", Gender::Neuter),
            Scale::Trillion if one => ("trilion", Gender::Neuter),
            Scale::Trillion => ("trilioane", Gender::Neuter),
        };
        let mut out = format!("{} {noun}", self.count_words(amount, Some(gender)));
        if noun_follows {
            out.push_str(" de");
        }
        out
    }

    /// `1,50 lei` "un leu și cincizeci de bani", `20 lei` "douăzeci de lei",
    /// `3 mil. lei` "trei milioane de lei".
    fn money_words(
        &self,
        amount: &Numeral,
        code: &str,
        per: Option<&UnitPart>,
        scale: Option<Scale>,
    ) -> String {
        let currency = TABLES
            .currencies
            .iter()
            .find(|c| c.code == code)
            .expect("code from the table");
        let major = Numeral::int(amount.integer);
        let minor_words = |cents: u128| -> String {
            let n = Numeral::int(cents);
            let name = if cents == 1 {
                currency.minor.0
            } else {
                currency.minor.1
            };
            format!("{} {name}", self.count_words(&n, currency.minor.2))
        };
        let mut out = String::new();
        match (scale, amount.fraction.len()) {
            (Some(scale), _) => {
                out.push_str(&self.scaled_words(amount, scale, true));
                out.push(' ');
                out.push_str(&self.head_noun(code, &Numeral::int(20)));
            }
            (None, 0) => {
                out.push_str(&self.count_words(&major, currency.gender));
                out.push(' ');
                out.push_str(&self.head_noun(code, &major));
            }
            (None, 1 | 2) => {
                let cents: u128 = format!("{:0<2}", amount.fraction).parse().unwrap_or(0);
                if amount.integer == 0 && cents > 0 {
                    out.push_str(&minor_words(cents));
                } else {
                    out.push_str(&self.count_words(&major, currency.gender));
                    out.push(' ');
                    out.push_str(&self.head_noun(code, &major));
                    if cents > 0 {
                        out.push_str(" și ");
                        out.push_str(&minor_words(cents));
                    }
                }
            }
            _ => {
                out.push_str(&words(amount, None));
                out.push(' ');
                out.push_str(&self.head_noun(code, amount));
            }
        }
        if amount.negative {
            out.insert_str(0, "minus ");
        }
        if let Some(part) = per {
            out.push(' ');
            out.push_str(self.units.per());
            out.push(' ');
            out.push_str(&self.units.per_name(&part.unit));
        }
        out
    }

    /// `1/2` "o jumătate", `3/4` "trei sferturi", `1 ½` "unu și jumătate".
    fn fraction_words(&self, whole: Option<&Numeral>, numerator: u32, denominator: u32) -> String {
        // Table for 2–12, 100, 1000; otherwise the cardinal with "-ime"/"-imi"
        // ("treisprezecime", "douăzecimi"), feminine like the table's nouns.
        let derived;
        let (singular, plural) = match TABLES.fractions.iter().find(|(d, _, _)| *d == denominator) {
            Some((_, s, p)) => (*s, *p),
            None => {
                let stem = words(&Numeral::from(denominator), None);
                let stem = stem
                    .strip_suffix(['e', 'i', 'ă'])
                    .unwrap_or(&stem)
                    .to_string();
                derived = (format!("{stem}ime"), format!("{stem}imi"));
                (derived.0.as_str(), derived.1.as_str())
            }
        };
        let gender = fraction_gender(denominator);
        let mut out = String::new();
        if let Some(whole) = whole {
            out.push_str(&words(whole, None));
            out.push_str(" și ");
            if numerator == 1 {
                out.push_str(singular);
                return out;
            }
        }
        let n = Numeral::from(numerator);
        out.push_str(&self.count_words(&n, Some(gender)));
        out.push(' ');
        out.push_str(if numerator == 1 { singular } else { plural });
        out
    }

    fn date_words(&self, day: Option<u8>, month: u8, year: Option<u64>) -> String {
        let mut out = String::new();
        if let Some(day) = day {
            if day == 1 {
                out.push_str("întâi");
            } else {
                out.push_str(&words(&day.into(), Some(Gender::Masculine)));
            }
            out.push(' ');
        }
        out.push_str(self.data.months_wide[usize::from(month) - 1]);
        if let Some(year) = year {
            out.push(' ');
            out.push_str(&year_words(year));
        }
        out
    }

    fn math_words(&self, items: &[MathItem]) -> String {
        let mut out = Vec::with_capacity(items.len());
        let mut last_number: Option<&Numeral> = None;
        for item in items {
            let word = match item {
                MathItem::Number(n) => {
                    last_number = Some(n);
                    words(n, None)
                }
                MathItem::Variable('π') | MathItem::Pi => "pi".to_string(),
                MathItem::Variable(c) => c.to_string(),
                MathItem::Word(w) => w.clone(),
                MathItem::Infinity => "infinit".to_string(),
                MathItem::Percent => "la sută".to_string(),
                MathItem::Unit(unit) => {
                    self.unit_or_percent(unit, last_number.unwrap_or(&Numeral::int(2)))
                }
                MathItem::Operator(op) => match op {
                    Operator::Plus => "plus",
                    Operator::Minus => "minus",
                    Operator::Times => "ori",
                    Operator::DividedBy => "împărțit la",
                    Operator::Equals => "egal",
                    Operator::NotEquals => "diferit de",
                    Operator::Less => "mai mic decât",
                    Operator::Greater => "mai mare decât",
                    Operator::LessOrEqual => "mai mic sau egal cu",
                    Operator::GreaterOrEqual => "mai mare sau egal cu",
                    Operator::Approximately => "aproximativ",
                    Operator::PlusMinus => "plus minus",
                    Operator::SquareRoot => "radical din",
                }
                .to_string(),
            };
            out.push(word);
        }
        out.join(" ")
    }

    /// Percent and permille read "la sută"/"la mie", not CLDR's "procente".
    fn unit_or_percent(&self, unit: &Unit, n: &Numeral) -> String {
        match &unit.numerator {
            Some(UnitPart {
                unit:
                    UnitRef::Cldr {
                        id: "concentr-percent",
                        ..
                    },
                ..
            }) if unit.denominators.is_empty() => "la sută".into(),
            Some(UnitPart {
                unit:
                    UnitRef::Cldr {
                        id: "concentr-permille",
                        ..
                    },
                ..
            }) if unit.denominators.is_empty() => "la mie".into(),
            _ => unit_words(self, unit, n),
        }
    }

    fn unit_gender(&self, unit: &Unit) -> Option<Gender> {
        unit.numerator
            .as_ref()
            .and_then(|p| self.units.gender(&p.unit))
    }
}

pub(super) fn verbalize(r: &Romanian, token: &Token, agreement: Agreement) -> Verbalized {
    let mut fallback = false;
    let spoken = match token {
        Token::Cardinal(n) => {
            if agreement.noun == NounPosition::After {
                r.count_words(n, agreement.gender)
            } else {
                words(n, agreement.gender)
            }
        }
        Token::Decimal(n) => words(n, None),
        Token::Dotted(groups) => dotted_words(r, Ro, groups),
        Token::Electronic(written) => electronic_words(r, Ro, written),
        Token::Year { year, .. } => year_words(*year),
        Token::Ordinal(n) => ordinal_words(*n, agreement.gender.unwrap_or(Gender::Masculine)),
        Token::Money {
            amount,
            currency,
            per,
            scale,
        } => r.money_words(amount, currency, per.as_ref(), *scale),
        Token::Scaled {
            amount,
            scale,
            unit,
        } => {
            // The unit's plural form carries its own "de" ("de kilometri").
            let mut out = r.scaled_words(
                amount,
                *scale,
                unit.is_none() && agreement.noun == NounPosition::After,
            );
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&r.unit_or_percent(unit, &Numeral::int(20)));
            }
            out
        }
        Token::Percent { value, permille } => {
            format!(
                "{} {}",
                words(value, None),
                if *permille { "la mie" } else { "la sută" }
            )
        }
        Token::Degrees { value, scale } => {
            let unit = UnitRef::Cldr {
                id: scale.unwrap_or("angle-degree"),
                prefix: None,
            };
            format!(
                "{} {}",
                words(value, r.units.gender(&unit)),
                r.units.name(&unit, value, Case::Nominative)
            )
        }
        Token::Measure {
            value,
            unit,
            hyphenated,
        } => {
            let joiner = if *hyphenated { "-" } else { " " };
            format!(
                "{}{joiner}{}",
                words(value, r.unit_gender(unit)),
                r.unit_or_percent(unit, value)
            )
        }
        Token::Time(clock) => r.clock_words(*clock),
        Token::TimeRange { from, to } => {
            format!("{} până la {}", r.clock_words(*from), r.clock_words(*to))
        }
        Token::Date { day, month, year } => r.date_words(*day, *month, *year),
        Token::Range { from, to } => {
            // The noun after the range governs both ends' gender ("două
            // până la trei ore"); only the end next to it takes the "de"
            // (§7.5: "douăzeci până la treizeci de minute"), and the far
            // end keeps the pronoun form of one ("una", not the article "o").
            let gender = agreement.gender;
            let near = agreement.noun == NounPosition::After;
            let end = |end: &RangeEnd, near: bool| match end {
                RangeEnd::Cardinal(n) if near => r.count_words(n, gender),
                RangeEnd::Cardinal(n) if n.integer == 1 => ruleset_words(1, gender),
                RangeEnd::Cardinal(n) => words(n, gender),
                RangeEnd::Ordinal(n) => ordinal_words(*n, Gender::Masculine),
                RangeEnd::Year(y) => year_words(*y),
            };
            format!("{} până la {}", end(from, false), end(to, near))
        }
        Token::Score { left, right } => format!(
            "{} la {}",
            words(&(*left).into(), None),
            words(&(*right).into(), None)
        ),
        Token::Telephone {
            groups,
            international,
        } => {
            let mut out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(Ro, digits))
                .collect();
            if *international {
                out[0].insert_str(0, "plus ");
            }
            out.join(", ")
        }
        Token::DigitGroups(groups) => {
            let out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(Ro, digits))
                .collect();
            out.join(", ")
        }
        Token::Digits(digits) => {
            fallback = true;
            spell::digits(Ro, digits)
        }
        Token::Abbreviation(key) => r.expansion(key).unwrap_or(key).to_string(),
        Token::Fraction {
            whole,
            numerator,
            denominator,
        } => r.fraction_words(whole.as_ref(), *numerator, *denominator),
        Token::Paragraph { plural } => {
            format!("{} ", if *plural { "paragrafele" } else { "paragraful" })
        }
        Token::Roman { from, to } => {
            let word = |n: u8| words(&n.into(), None);
            match to {
                Some(to) => format!("{} până la {}", word(*from), word(*to)),
                None => word(*from),
            }
        }
        Token::Scientific {
            mantissa,
            exponent,
            unit,
        } => {
            let mut out = format!(
                "{} ori zece la puterea {}",
                words(mantissa, None),
                words(&signed_int(*exponent), None)
            );
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(r, unit, mantissa));
            }
            out
        }
        Token::Power { base, exponent } => {
            let base = match base {
                PowerBase::Number(n) => words(n, None),
                PowerBase::Variable('π') => "pi".to_string(),
                PowerBase::Variable(c) => c.to_string(),
            };
            match exponent {
                2 => format!("{base} la pătrat"),
                3 => format!("{base} la cub"),
                n => format!("{base} la puterea {}", words(&signed_int(*n), None)),
            }
        }
        Token::Chemical(parts) => chemical_words(r, parts),
        Token::Math(items) => r.math_words(items),
        Token::Ratio {
            left,
            right,
            unit,
            kind,
        } => {
            let joiner = match kind {
                RatioKind::BloodPressure => "cu",
                RatioKind::Ratio => "la",
                RatioKind::Range => "până la",
            };
            let mut out = format!("{} {joiner} {}", words(left, None), words(right, None));
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(r, unit, right));
            }
            out
        }
        Token::Dose { groups, unit } => {
            let mut out = groups
                .iter()
                .map(|n| words(n, None))
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(
                    r,
                    unit,
                    groups.last().unwrap_or(&Numeral::int(2)),
                ));
            }
            out
        }
        Token::Repetition { count, per } => {
            // "o dată", "de două ori", "de douăzeci de ori"
            let mut out = if count.is_integer() && count.integer == 1 {
                "o dată".to_string()
            } else {
                format!(
                    "de {} ori",
                    r.count_words(count, count.is_integer().then_some(Gender::Feminine))
                )
            };
            if let Some(unit) = per {
                out.push(' ');
                out.push_str(&unit_words(r, unit, count));
            }
            out
        }
        Token::Angle {
            degrees,
            minutes,
            seconds,
            compass,
        } => {
            let name = |id: &'static str, n: &Numeral| {
                r.units
                    .name(&UnitRef::Cldr { id, prefix: None }, n, Case::Nominative)
            };
            let mut out = format!(
                "{} {}",
                words(degrees, Some(Gender::Neuter)),
                name("angle-degree", degrees)
            );
            if let Some(minutes) = minutes {
                out.push_str(&format!(
                    " {} {}",
                    words(minutes, Some(Gender::Neuter)),
                    name("duration-minute", minutes)
                ));
            }
            if let Some(seconds) = seconds {
                out.push_str(&format!(
                    " {} {}",
                    words(seconds, Some(Gender::Feminine)),
                    name("duration-second", seconds)
                ));
            }
            if let Some(c) = compass {
                if let Some((_, word)) = TABLES.compass.iter().find(|(l, _)| l == c) {
                    out.push(' ');
                    out.push_str(word);
                }
            }
            out
        }
        Token::Lexicon(key) => key.clone(),
    };
    Verbalized { spoken, fallback }
}
