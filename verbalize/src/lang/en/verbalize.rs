//! English spoken forms. Readings not fixed by CLDR are chosen here
//! and recorded in `docs/rules/en.md`: "two thirty p m", "nine o'clock",
//! "fourteen thirty", "two to one", "zero" for telephone digits.

use super::lexicon::*;
use super::English;
use crate::lang::common::{chemical_words, dotted_words, electronic_words, signed_int, unit_words};
use crate::lang::Verbalized;
use crate::rbnf::Value;
use crate::spell;
use crate::token::*;
use crate::Language::En;

fn value(n: &Numeral) -> Value<'_> {
    Value::decimal(n.negative, n.integer, &n.fraction)
}

/// Cardinal words via `%spellout-numbering` ("one point five" for decimals).
pub(super) fn words(n: &Numeral) -> String {
    let words = spell::spell(En, "%spellout-numbering", value(n));
    if words.bytes().any(|b| b.is_ascii_digit()) {
        let mut digits = spell::digits(En, &n.integer.to_string());
        if !n.fraction.is_empty() {
            digits.push_str(" point ");
            digits.push_str(&spell::digits(En, &n.fraction));
        }
        return if n.negative {
            format!("minus {digits}")
        } else {
            digits
        };
    }
    words
}

fn ordinal_words(n: u64) -> String {
    spell::spell(En, "%spellout-ordinal", Value::int(i128::from(n)))
}

fn year_words(year: u64) -> String {
    spell::spell(En, "%spellout-numbering-year", Value::int(i128::from(year)))
}

fn meridiem_words(m: Meridiem) -> &'static str {
    match m {
        Meridiem::Am => "a m",
        Meridiem::Pm => "p m",
    }
}

/// `9:00` "nine o'clock", `9:05` "nine oh five", `14:30` "fourteen thirty",
/// `14:00` "fourteen hundred", `2:30 pm` "two thirty p m", `2 pm` "two p m";
/// bare `hh:mm:ss` reads as counted nouns: `1:01:01` "one hour one minute
/// and one second"; a meridiem or a recognised time zone after it makes the
/// reading unambiguously a clock instead, seconds appended when nonzero:
/// `2:30:15 pm` "two thirty and fifteen seconds p m", `10:00:00 p.m. EST`
/// "ten p m EST".
fn clock_words(clock: Clock, with_meridiem: bool) -> String {
    let hour = words(&clock.hour.into());
    let counted = |n: u8, one: &str, many: &str| {
        format!("{} {}", words(&n.into()), if n == 1 { one } else { many })
    };
    let duration = clock.second.is_some() && clock.meridiem.is_none() && !clock.is_clock;
    let mut out = if duration {
        format!(
            "{} {} and {}",
            counted(clock.hour, "hour", "hours"),
            counted(clock.minute.unwrap_or(0), "minute", "minutes"),
            counted(clock.second.unwrap_or(0), "second", "seconds")
        )
    } else {
        let mut base = match clock.minute {
            None | Some(0) if clock.meridiem.is_some() => hour.clone(),
            None => hour.clone(),
            Some(0) if clock.hour > 12 => format!("{hour} hundred"),
            Some(0) => format!("{hour} o'clock"),
            Some(m @ 1..=9) => format!("{hour} oh {}", words(&m.into())),
            Some(m) => format!("{hour} {}", words(&m.into())),
        };
        if let Some(s) = clock.second.filter(|s| *s > 0) {
            base.push_str(&format!(" and {}", counted(s, "second", "seconds")));
        }
        base
    };
    if let (true, Some(m)) = (with_meridiem, clock.meridiem) {
        out.push(' ');
        out.push_str(meridiem_words(m));
    }
    out
}

impl English {
    fn head_noun(&self, code: &str, n: &Numeral) -> String {
        let cldr = self.data.currencies.iter().find(|c| c.code == code);
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
    }

    /// `2.5 billion` → "two point five billion", `5k` → "five thousand".
    fn scaled_words(&self, amount: &Numeral, scale: Scale) -> String {
        let noun = match scale {
            Scale::Thousand => "thousand",
            Scale::Million => "million",
            Scale::Billion => "billion",
            Scale::Trillion => "trillion",
        };
        format!("{} {noun}", words(amount))
    }

    /// `$8.80` "eight dollars and eighty cents", `$0.99` "ninety-nine cents",
    /// `$1.01` "one dollar and one cent", `$2.5 billion` "two point five
    /// billion dollars".
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
        let mut out = String::new();
        match (scale, amount.fraction.len()) {
            (Some(scale), _) => {
                out.push_str(&self.scaled_words(amount, scale));
                out.push(' ');
                out.push_str(&self.head_noun(code, &Numeral::int(20)));
            }
            (None, 0) => {
                out.push_str(&words(&major));
                out.push(' ');
                out.push_str(&self.head_noun(code, &major));
            }
            (None, 1 | 2) => {
                let cents: u128 = format!("{:0<2}", amount.fraction).parse().unwrap_or(0);
                let minor = if cents == 1 {
                    currency.minor.0
                } else {
                    currency.minor.1
                };
                if amount.integer == 0 && cents > 0 {
                    out.push_str(&words(&Numeral::int(cents)));
                    out.push(' ');
                    out.push_str(minor);
                } else {
                    out.push_str(&words(&major));
                    out.push(' ');
                    out.push_str(&self.head_noun(code, &major));
                    if cents > 0 {
                        out.push_str(" and ");
                        out.push_str(&words(&Numeral::int(cents)));
                        out.push(' ');
                        out.push_str(minor);
                    }
                }
            }
            _ => {
                out.push_str(&words(amount));
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

    /// `1/2` "one half", `3/4` "three quarters", `1 ½` "one and a half".
    fn fraction_words(&self, whole: Option<&Numeral>, numerator: u32, denominator: u32) -> String {
        // Table for 2–12, 100, 1000; otherwise the ordinal ("thirty-seconds").
        let derived;
        let name = match TABLES.fractions.iter().find(|(d, _, _)| *d == denominator) {
            Some((_, s, p)) if numerator == 1 => *s,
            Some((_, _, p)) => *p,
            None => {
                derived =
                    ordinal_words(u64::from(denominator)) + if numerator == 1 { "" } else { "s" };
                derived.as_str()
            }
        };
        let mut out = String::new();
        if let Some(whole) = whole {
            out.push_str(&words(whole));
            out.push_str(" and ");
            if numerator == 1 {
                out.push_str("a ");
                out.push_str(name);
                return out;
            }
        }
        out.push_str(&words(&Numeral::from(numerator)));
        out.push(' ');
        out.push_str(name);
        out
    }

    /// Month-name forms keep their written order: `November 1` "November
    /// first", `1 November` "the first of November". Numeric and ISO
    /// dates read month first in en-US and "the first of November" in en-GB;
    /// a year-month (`2003-03`) is the month name and the year.
    fn date_words(
        &self,
        day: Option<u8>,
        month: u8,
        year: Option<u64>,
        noun: NounPosition,
    ) -> String {
        let month_name = self.data.months_wide[usize::from(month) - 1];
        let month_first = match noun {
            NounPosition::Before => true,
            NounPosition::After => false,
            NounPosition::Absent => !self.day_first,
        };
        let mut out = match day.map(|d| ordinal_words(u64::from(d))) {
            None => month_name.to_string(),
            Some(day_words) if month_first => format!("{month_name} {day_words}"),
            Some(day_words) => format!("the {day_words} of {month_name}"),
        };
        if let Some(year) = year {
            out.push(' ');
            out.push_str(&year_words(year));
        }
        out
    }

    fn math_words(&self, items: &[MathItem]) -> String {
        let mut out = Vec::with_capacity(items.len());
        let mut last_number: Option<&Numeral> = None;
        for (i, item) in items.iter().enumerate() {
            let word = match item {
                MathItem::Number(n) => {
                    last_number = Some(n);
                    words(n)
                }
                MathItem::Variable('π') | MathItem::Pi => "pi".to_string(),
                MathItem::Variable(c) => c.to_string(),
                MathItem::Word(w) => w.clone(),
                MathItem::Infinity => "infinity".to_string(),
                MathItem::Percent => "percent".to_string(),
                MathItem::Unit(unit) => {
                    unit_words(self, unit, last_number.unwrap_or(&Numeral::int(2)))
                }
                MathItem::Operator(op) => {
                    // A comparison with a left operand reads "is …".
                    let is = if i > 0 { "is " } else { "" };
                    match op {
                        Operator::Plus => "plus".to_string(),
                        Operator::Minus => "minus".to_string(),
                        Operator::Times => "times".to_string(),
                        Operator::DividedBy => "divided by".to_string(),
                        Operator::Equals => "equals".to_string(),
                        Operator::NotEquals => format!("{is}not equal to"),
                        Operator::Less => format!("{is}less than"),
                        Operator::Greater => format!("{is}greater than"),
                        Operator::LessOrEqual => format!("{is}less than or equal to"),
                        Operator::GreaterOrEqual => format!("{is}greater than or equal to"),
                        Operator::Approximately => format!("{is}approximately"),
                        Operator::PlusMinus => "plus or minus".to_string(),
                        Operator::SquareRoot => "the square root of".to_string(),
                    }
                }
            };
            out.push(word);
        }
        out.join(" ")
    }
}

pub(super) fn verbalize(e: &English, token: &Token, agreement: Agreement) -> Verbalized {
    let mut fallback = false;
    let spoken = match token {
        Token::Cardinal(n) | Token::Decimal(n) => words(n),
        Token::Dotted(groups) => dotted_words(e, En, groups),
        Token::Electronic(written) => electronic_words(e, En, written),
        Token::Year { year, decade } => {
            let mut out = year_words(*year);
            if *decade {
                // "the nineties", "the nineteen hundreds", "the two thousands"
                if out.ends_with('y') {
                    out.pop();
                    out.push_str("ies");
                } else {
                    out.push('s');
                }
            }
            out
        }
        Token::Ordinal(n) => ordinal_words(*n),
        Token::Money {
            amount,
            currency,
            per,
            scale,
        } => e.money_words(amount, currency, per.as_ref(), *scale),
        Token::Scaled {
            amount,
            scale,
            unit,
        } => {
            let mut out = e.scaled_words(amount, *scale);
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(e, unit, &Numeral::int(20)));
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
                words(value),
                e.units.name(&unit, value, Case::Nominative)
            )
        }
        Token::Degrees { value, scale } => {
            let unit = UnitRef::Cldr {
                id: scale.unwrap_or("angle-degree"),
                prefix: None,
            };
            format!(
                "{} {}",
                words(value),
                e.units.name(&unit, value, Case::Nominative)
            )
        }
        Token::Measure {
            value,
            unit,
            hyphenated: false,
        } => format!("{} {}", words(value), unit_words(e, unit, value)),
        // Attributive compounds take the singular: "a seventy-five-gram test".
        Token::Measure {
            value,
            unit,
            hyphenated: true,
        } => {
            format!("{}-{}", words(value), unit_words(e, unit, &Numeral::int(1)))
        }
        Token::Time(clock) => clock_words(*clock, true),
        Token::TimeRange { from, to } => {
            // The meridiem is read once, after the second time, as written.
            let shared = from.meridiem == to.meridiem;
            format!(
                "{} to {}",
                clock_words(*from, !shared),
                clock_words(*to, true)
            )
        }
        Token::Date { day, month, year } => e.date_words(*day, *month, *year, agreement.noun),
        Token::Range { from, to, unit } => {
            let end = |end: &RangeEnd| match end {
                RangeEnd::Cardinal(n) => words(n),
                RangeEnd::Ordinal(n) => ordinal_words(*n),
                RangeEnd::Year(y) => year_words(*y),
            };
            let mut out = format!("{} to {}", end(from), end(to));
            if let (Some(unit), RangeEnd::Cardinal(n)) = (unit, to) {
                out.push(' ');
                out.push_str(&unit_words(e, unit, n));
            }
            out
        }
        Token::Score { left, right } => {
            format!("{} to {}", words(&(*left).into()), words(&(*right).into()))
        }
        Token::Telephone {
            groups,
            international,
        } => {
            let mut out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(En, digits))
                .collect();
            if *international {
                out[0].insert_str(0, "plus ");
            }
            out.join(", ")
        }
        Token::DigitGroups(groups) => {
            let out: Vec<String> = groups
                .iter()
                .map(|digits| spell::digits(En, digits))
                .collect();
            out.join(", ")
        }
        Token::Digits(digits) => {
            fallback = true;
            spell::digits(En, digits)
        }
        Token::Abbreviation(key) => {
            if key == "St." && agreement.noun == NounPosition::After {
                SAINT.to_string()
            } else {
                e.expansion(key).unwrap_or(key).to_string()
            }
        }
        Token::Fraction {
            whole,
            numerator,
            denominator,
        } => e.fraction_words(whole.as_ref(), *numerator, *denominator),
        Token::Paragraph { plural } => format!("{} ", if *plural { "sections" } else { "section" }),
        Token::Roman { from, to } => {
            let word = |n: u8| words(&n.into());
            match to {
                Some(to) => format!("{} to {}", word(*from), word(*to)),
                None => word(*from),
            }
        }
        Token::Scientific {
            mantissa,
            exponent,
            unit,
        } => {
            let mut out = format!(
                "{} times ten to the power of {}",
                words(mantissa),
                words(&signed_int(*exponent))
            );
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(e, unit, mantissa));
            }
            out
        }
        Token::Power { base, exponent } => {
            let base = match base {
                PowerBase::Number(n) => words(n),
                PowerBase::Variable('π') => "pi".to_string(),
                PowerBase::Variable(c) => c.to_string(),
            };
            match exponent {
                2 => format!("{base} squared"),
                3 => format!("{base} cubed"),
                n => format!("{base} to the power of {}", words(&signed_int(*n))),
            }
        }
        Token::Chemical(parts) => chemical_words(e, parts),
        Token::Math(items) => e.math_words(items),
        Token::Ratio {
            left,
            right,
            unit,
            kind,
        } => {
            let joiner = if *kind == RatioKind::BloodPressure {
                "over"
            } else {
                "to"
            };
            let mut out = format!("{} {joiner} {}", words(left), words(right));
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(e, unit, right));
            }
            out
        }
        Token::Dose { groups, unit } => {
            let mut out = groups.iter().map(words).collect::<Vec<_>>().join(", ");
            if let Some(unit) = unit {
                out.push(' ');
                out.push_str(&unit_words(
                    e,
                    unit,
                    groups.last().unwrap_or(&Numeral::int(2)),
                ));
            }
            out
        }
        Token::Repetition { count, per } => {
            let mut out = match (count.is_integer(), count.integer) {
                (true, 1) => "once".to_string(),
                (true, 2) => "twice".to_string(),
                _ => format!("{} times", words(count)),
            };
            if let Some(unit) = per {
                out.push(' ');
                out.push_str(&unit_words(e, unit, count));
            }
            out
        }
        Token::Angle {
            degrees,
            minutes,
            seconds,
            compass,
        } => {
            let mut out = format!("{} degrees", words(degrees));
            if let Some(minutes) = minutes {
                out.push_str(&format!(" {} minutes", words(minutes)));
            }
            if let Some(seconds) = seconds {
                out.push_str(&format!(" {} seconds", words(seconds)));
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
