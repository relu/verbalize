//! German spoken forms: every token to words, via the RBNF rulesets for
//! numbers and the CLDR tables for names.

use super::lexicon::*;
use super::German;
use crate::lang::common::{chemical_words, dotted_words, electronic_words, signed_int, unit_words};
use crate::lang::Verbalized;
use crate::rbnf::Value;
use crate::spell;
use crate::token::*;
use crate::Language::De;

const NUMBERING: &str = "%spellout-numbering";

fn value(n: &Numeral) -> Value<'_> {
    Value::decimal(n.negative, n.integer, &n.fraction)
}

/// Cardinal words; a decimal reads via the numbering ruleset's `x.x`
/// rule, an integer with a known noun gender via the gendered ruleset.
pub(super) fn words(n: &Numeral, gender: Option<Gender>) -> String {
    let ruleset = if n.is_integer() {
        spell::cardinal_ruleset(
            De,
            Agreement {
                gender,
                ..Agreement::default()
            },
        )
    } else {
        NUMBERING
    };
    let words = spell::spell(De, ruleset, value(n));
    if words.bytes().any(|b| b.is_ascii_digit()) {
        // Beyond the rules (10^18 and up) ICU falls back to digits; we
        // read them one by one instead (§9: no digits in the output).
        let mut digits = spell::digits(De, &n.integer.to_string());
        if !n.fraction.is_empty() {
            digits.push_str(" Komma ");
            digits.push_str(&spell::digits(De, &n.fraction));
        }
        return if n.negative {
            format!("minus {digits}")
        } else {
            digits
        };
    }
    words
}

fn ordinal_words(n: u64, agreement: Agreement) -> String {
    spell::spell(
        De,
        spell::ordinal_ruleset(De, agreement),
        Value::int(i128::from(n)),
    )
}

fn year_words(year: u64) -> String {
    spell::spell(De, "%spellout-numbering-year", Value::int(i128::from(year)))
}

impl German {
    fn clock_words(&self, clock: Clock) -> String {
        // "ein Uhr", not "eins Uhr" (Duden).
        let mut out = if clock.hour == 1 {
            "ein".to_string()
        } else {
            words(&clock.hour.into(), None)
        };
        out.push_str(" Uhr");
        if let Some(second) = clock.second {
            // `hh:mm:ss`: minutes and seconds are counted nouns.
            let counted = |n: u8, one: &str, other: &str| {
                let n_words = words(&n.into(), Some(Gender::Feminine));
                format!(" {n_words} {}", if n == 1 { one } else { other })
            };
            out.push_str(&counted(clock.minute.unwrap_or(0), "Minute", "Minuten"));
            out.push_str(&counted(second, "Sekunde", "Sekunden"));
            return out;
        }
        if let Some(minute) = clock.minute.filter(|m| *m > 0) {
            out.push(' ');
            out.push_str(&words(&minute.into(), None));
        }
        out
    }

    fn range_end_words(&self, end: &RangeEnd, agreement: Agreement) -> String {
        match end {
            RangeEnd::Cardinal(n) => words(n, None),
            RangeEnd::Ordinal(n) => ordinal_words(*n, agreement),
            RangeEnd::Year(y) => year_words(*y),
        }
    }

    /// `2,5 Mrd.` → "zwei Komma fünf Milliarden", `1 Mio.` → "eine Million"
    /// (the scale nouns are feminine), `3 Tsd.` → "drei Tausend".
    fn scaled_words(&self, amount: &Numeral, scale: Scale) -> String {
        let one = amount.is_integer() && amount.integer == 1 && !amount.negative;
        let (noun, gender) = match scale {
            Scale::Thousand => ("Tausend", None),
            Scale::Million if one => ("Million", Some(Gender::Feminine)),
            Scale::Million => ("Millionen", Some(Gender::Feminine)),
            Scale::Billion if one => ("Milliarde", Some(Gender::Feminine)),
            Scale::Billion => ("Milliarden", Some(Gender::Feminine)),
            Scale::Trillion if one => ("Billion", Some(Gender::Feminine)),
            Scale::Trillion => ("Billionen", Some(Gender::Feminine)),
        };
        format!("{} {noun}", words(amount, gender))
    }

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
        let cldr = self.data.currencies.iter().find(|c| c.code == code);
        let head_noun = |n: &Numeral| -> String {
            let category = self.units.category(n);
            let name = cldr
                .and_then(|c| {
                    c.names
                        .iter()
                        .find(|(p, _)| *p == category)
                        .map(|(_, n)| *n)
                })
                .or(cldr.map(|c| c.name))
                .unwrap_or(code);
            name.rsplit([' ', '-']).next().unwrap_or(name).to_string()
        };
        let major = Numeral::int(amount.integer);
        let mut out = String::new();
        // A scaled amount is never cents: "zwei Komma fünf Milliarden Euro".
        match (scale, amount.fraction.len()) {
            (None, 0) => {
                out.push_str(&words(&major, currency.gender));
                out.push(' ');
                out.push_str(&head_noun(&major));
            }
            (None, 1 | 2) => {
                let cents: u128 = format!("{:0<2}", amount.fraction).parse().unwrap_or(0);
                let (minor_one, minor_other, minor_gender) = currency.minor;
                if amount.integer == 0 && cents > 0 {
                    out.push_str(&words(&Numeral::int(cents), minor_gender));
                    out.push(' ');
                    out.push_str(if cents == 1 { minor_one } else { minor_other });
                } else {
                    out.push_str(&words(&major, currency.gender));
                    out.push(' ');
                    out.push_str(&head_noun(&major));
                    if cents > 0 {
                        out.push(' ');
                        out.push_str(&words(&Numeral::int(cents), None));
                    }
                }
            }
            (Some(scale), _) => {
                out.push_str(&self.scaled_words(amount, scale));
                out.push(' ');
                out.push_str(&head_noun(&Numeral::int(20)));
            }
            _ => {
                out.push_str(&words(amount, None));
                out.push(' ');
                out.push_str(&head_noun(amount));
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

    fn fraction_words(&self, whole: Option<&Numeral>, numerator: u32, denominator: u32) -> String {
        // Table for 2–12, 100, 1000; otherwise the cardinal plus "-tel"
        // (13–19) or "-stel" (20–99): "Dreizehntel", "Zwanzigstel" (Duden).
        let name = match TABLES.fractions.iter().find(|(d, _, _)| *d == denominator) {
            Some((_, n, _)) => (*n).to_string(),
            None => {
                let stem = words(&Numeral::from(denominator), None);
                let mut chars = stem.chars();
                let head: String = chars
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_default();
                let suffix = if denominator < 20 { "tel" } else { "stel" };
                format!("{head}{}{suffix}", chars.as_str())
            }
        };
        let name = name.as_str();
        let mut out = String::new();
        if let Some(whole) = whole {
            out.push_str(&words(whole, Some(Gender::Neuter)));
            out.push(' ');
            if numerator == 1 && denominator == 2 {
                out.push_str("einhalb");
                return out;
            }
        }
        out.push_str(&words(&Numeral::from(numerator), Some(Gender::Neuter)));
        out.push(' ');
        out.push_str(name);
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
                MathItem::Variable('π') | MathItem::Pi => "Pi".to_string(),
                MathItem::Variable(c) => c.to_string(),
                MathItem::Word(w) => w.clone(),
                MathItem::Infinity => "unendlich".to_string(),
                MathItem::Percent => "Prozent".to_string(),
                MathItem::Unit(unit) => {
                    unit_words(self, unit, last_number.unwrap_or(&Numeral::int(2)))
                }
                MathItem::Operator(op) => match op {
                    Operator::Plus => "plus",
                    Operator::Minus => "minus",
                    Operator::Times => "mal",
                    Operator::DividedBy => "geteilt durch",
                    Operator::Equals => "gleich",
                    Operator::NotEquals => "ungleich",
                    Operator::Less => "kleiner als",
                    Operator::Greater => "größer als",
                    Operator::LessOrEqual => "kleiner oder gleich",
                    Operator::GreaterOrEqual => "größer oder gleich",
                    Operator::Approximately => "ungefähr",
                    Operator::PlusMinus => "plus minus",
                    Operator::SquareRoot => "Wurzel aus",
                }
                .to_string(),
            };
            out.push(word);
        }
        out.join(" ")
    }

    fn unit_gender(&self, unit: &Unit) -> Option<Gender> {
        unit.numerator
            .as_ref()
            .and_then(|p| self.units.gender(&p.unit))
    }
}

pub(super) fn verbalize(g: &German, token: &Token, agreement: Agreement) -> Verbalized {
    let mut fallback = false;
    let spoken = match token {
        Token::Cardinal(n) => words(n, agreement.gender),
        Token::Year { year, decade } => {
            let mut out = year_words(*year);
            if *decade {
                out.push_str("er");
            }
            out
        }
        Token::Ordinal(n) => ordinal_words(*n, agreement),
        Token::Decimal(n) => words(n, None),
        Token::Dotted(groups) => dotted_words(g, De, groups),
        Token::Electronic(written) => electronic_words(g, De, written),
        Token::Money {
            amount,
            currency,
            per,
            scale,
        } => g.money_words(amount, currency, per.as_ref(), *scale),
        Token::Scaled {
            amount,
            scale,
            unit,
        } => {
            let mut out = g.scaled_words(amount, *scale);
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(g, unit, &Numeral::int(20)));
            }
            out
        }
        Token::Percent { value, permille } => {
            let unit = UnitRef::Cldr {
                id: if *permille {
                    "concentr-permille"
                } else {
                    "concentr-percent"
                },
                prefix: None,
            };
            format!(
                "{} {}",
                words(value, g.units.gender(&unit)),
                g.units.name(&unit, value, Case::Nominative)
            )
        }
        Token::Degrees { value, scale } => {
            let unit = UnitRef::Cldr {
                id: scale.unwrap_or("angle-degree"),
                prefix: None,
            };
            format!(
                "{} {}",
                words(value, g.units.gender(&unit)),
                g.units.name(&unit, value, Case::Nominative)
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
                words(value, g.unit_gender(unit)),
                unit_words(g, unit, value)
            )
        }
        Token::Time(clock) => g.clock_words(*clock),
        Token::TimeRange { from, to } => {
            if from.minute.is_none() && to.minute.is_none() {
                format!(
                    "{} bis {} Uhr",
                    words(&from.hour.into(), None),
                    words(&to.hour.into(), None)
                )
            } else {
                format!("{} bis {}", g.clock_words(*from), g.clock_words(*to))
            }
        }
        Token::Date { day, month, year } => {
            let mut out = String::new();
            if let Some(day) = day {
                out.push_str(&ordinal_words(u64::from(*day), agreement));
                out.push(' ');
            }
            out.push_str(g.data.months_wide[usize::from(*month) - 1]);
            if let Some(year) = year {
                out.push(' ');
                out.push_str(&year_words(*year));
            }
            out
        }
        Token::Range { from, to } => {
            format!(
                "{} bis {}",
                g.range_end_words(from, agreement),
                g.range_end_words(to, agreement)
            )
        }
        Token::Score { left, right } => format!(
            "{} zu {}",
            words(&(*left).into(), None),
            words(&(*right).into(), None)
        ),
        Token::Telephone {
            groups,
            international,
        } => {
            let mut out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(De, digits))
                .collect();
            if *international {
                out[0].insert_str(0, "plus ");
            }
            out.join(", ")
        }
        Token::DigitGroups(groups) => {
            let out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(De, digits))
                .collect();
            out.join(", ")
        }
        Token::Digits(digits) => {
            fallback = true;
            spell::digits(De, digits)
        }
        Token::Abbreviation(key) => {
            if key == "Fr." && agreement.noun == NounPosition::After {
                FRAU.to_string()
            } else {
                g.expansion(key).unwrap_or(key).to_string()
            }
        }
        Token::Fraction {
            whole,
            numerator,
            denominator,
        } => g.fraction_words(whole.as_ref(), *numerator, *denominator),
        Token::Paragraph { plural } => {
            format!("{} ", if *plural { "Paragrafen" } else { "Paragraf" })
        }
        Token::Roman { from, to } => {
            let word = |n: u8| words(&n.into(), None);
            match to {
                Some(to) => format!("{} bis {}", word(*from), word(*to)),
                None => word(*from),
            }
        }
        Token::Scientific {
            mantissa,
            exponent,
            unit,
        } => {
            let mut out = format!(
                "{} mal zehn hoch {}",
                words(mantissa, None),
                words(&signed_int(*exponent), None)
            );
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(g, unit, mantissa));
            }
            out
        }
        Token::Power { base, exponent } => {
            let base = match base {
                PowerBase::Number(n) => words(n, None),
                PowerBase::Variable('π') => "Pi".to_string(),
                PowerBase::Variable(c) => c.to_string(),
            };
            format!("{base} hoch {}", words(&signed_int(*exponent), None))
        }
        Token::Chemical(parts) => chemical_words(g, parts),
        Token::Math(items) => g.math_words(items),
        Token::Ratio {
            left,
            right,
            unit,
            kind,
        } => {
            let joiner = if *kind == RatioKind::Range {
                "bis"
            } else {
                "zu"
            };
            let mut out = format!("{} {joiner} {}", words(left, None), words(right, None));
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(g, unit, right));
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
                    g,
                    unit,
                    groups.last().unwrap_or(&Numeral::int(2)),
                ));
            }
            out
        }
        Token::Repetition { count, per } => {
            let mut out = if count.is_integer() {
                format!("{}mal", words(count, Some(Gender::Masculine)))
            } else {
                format!("{} mal", words(count, None))
            };
            if let Some(unit) = per {
                out.push(' ');
                out.push_str(&unit_words(g, unit, count));
            }
            out
        }
        Token::Angle {
            degrees,
            minutes,
            seconds,
            compass,
        } => {
            let mut out = format!("{} Grad", words(degrees, None));
            if let Some(minutes) = minutes {
                out.push_str(&format!(" {} Minuten", words(minutes, None)));
            }
            if let Some(seconds) = seconds {
                out.push_str(&format!(" {} Sekunden", words(seconds, None)));
            }
            if let Some(c) = compass {
                if let Some((_, name)) = TABLES.compass.iter().find(|(l, _)| l == c) {
                    out.push(' ');
                    out.push_str(name);
                }
            }
            out
        }
        Token::Lexicon(key) => key.clone(),
    };
    Verbalized { spoken, fallback }
}
