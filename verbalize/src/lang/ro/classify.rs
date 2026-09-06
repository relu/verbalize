//! Romanian recognisers in priority order. The language-neutral ones come
//! from [`common`]; times, dates and the `al N-lea`/`a N-a` ordinals are
//! Romanian shapes.

use regex::Regex;

use super::lexicon::TABLES;
use super::Romanian;
use crate::lang::common::{self, group, iso_bounded, recognizer, text_of, Patterns};
use crate::lang::{char_after, char_before, digit_bounded, Context};
use crate::pipeline::{Caps, Match, Recognizer};
use crate::token::*;

pub(super) fn recognizers(months: &str, abbreviations: Option<Regex>) -> Vec<Recognizer<Romanian>> {
    let p = Patterns::new(&super::NUMERALS, &TABLES);
    let num = super::NUMERALS.num();
    let plain = super::NUMERALS.plain();
    let mut list = vec![
        // `@`, `://`, `www.` and a known TLD are unambiguous: above everything.
        recognizer(&p.electronic, common::electronic),
        recognizer(
            r"([0-9]{1,2}):([0-9]{2})(\s*[–-]\s*|\s+(?:până\s+)?la\s+)([0-9]{1,2}):([0-9]{2})",
            time_range,
        ),
        recognizer(r"(?:(ora|orele)\s+)?([0-9]{1,2})(?::([0-9]{2}))?", time),
        recognizer(
            &format!(
                r"(?:([0-9]{{1,2}})\.([0-9]{{1,2}})\.([0-9]{{4}})?|([0-9]{{4}})-([0-9]{{2}})(?:-([0-9]{{2}}))?|([0-9]{{1,2}})\s+({months})(?:\s+([0-9]{{4}}))?)"
            ),
            date,
        ),
        recognizer(&p.iban, common::iban),
        recognizer(&p.card, common::digit_groups),
        recognizer(
            r"(\+[0-9]{1,3}|\(0[0-9]+\)|0[0-9]+)((?:[ /\-.][0-9]+)+)",
            common::telephone,
        ),
        // `al 3-lea`, `a 5-a`, `al II-lea`: the suffix is the marker, so the
        // ordinal outranks the classes that would take its letters (chemical,
        // math variables) and its digits.
        recognizer(r"(?:(al|a|Al|A)\s+)?([0-9]+|[IVXLCDM]+)-(lea|a)", ordinal),
        recognizer(&p.scientific, common::scientific),
        recognizer(&p.chemical, common::chemical),
        recognizer(&p.power, common::power),
        recognizer(&p.zone_offset, common::zone_offset),
        recognizer(&p.math, common::math),
        recognizer(&p.angle, common::angle),
        recognizer(&p.blood_pressure, common::blood_pressure),
        recognizer(&p.range_rate, common::range_rate),
        recognizer(&p.ratio, common::ratio),
        recognizer(&p.dose, common::dose),
        recognizer(
            &format!(
                r"(?:(€|\$|£)\s?)?(-?{num})(?:\s*(?:de\s+)?(mii|milioane|milion|mil\.|miliarde|miliard|mld\.))?(?:\s*(?:de\s+)?(lei|leu|Lei|LEI|RON|ron|€|EUR|euro|\$|USD|dolari|dolar|£|GBP|lire|liră))?(?:/(\p{{L}}[\p{{L}}.]*))?"
            ),
            common::money,
        ),
        recognizer(&p.percent, common::percent),
        recognizer(&p.degrees, common::degrees),
        recognizer(&p.measure, common::measure),
        recognizer(&p.hyphenated_measure, common::hyphenated_measure),
        recognizer(
            &format!(r"(-?{plain})\s*[x×](/\p{{L}}+)?"),
            common::repetition,
        ),
        recognizer(&p.range, common::range),
        recognizer(&p.dotted, common::dotted),
        recognizer(&p.slash_fraction, common::slash_fraction),
        recognizer(&p.year, common::year),
        recognizer(&p.decimal, common::decimal),
        recognizer(&p.mixed_fraction, common::mixed_fraction),
        recognizer(&p.score, common::score),
        recognizer(&p.cardinal_grouped, common::cardinal),
        recognizer(&p.cardinal_plain, common::cardinal),
        recognizer(&p.unicode_fraction, common::unicode_fraction),
        recognizer(&p.roman, common::roman),
        recognizer(&p.paragraph, common::paragraph),
    ];
    if let Some(regex) = abbreviations {
        list.push(Recognizer {
            regex,
            parse: common::abbreviation,
        });
    }
    list
}

fn u8_at(text: &str) -> Option<u8> {
    text.parse().ok()
}

/// `Some(None)` when group `i` did not participate, `None` on a bad number.
fn optional_u8(caps: &Caps<'_>, i: usize) -> Option<Option<u8>> {
    match caps.get(i) {
        Some(_) => u8_at(text_of(caps, i)).map(Some),
        None => Some(None),
    }
}

fn clock(hour: &str, minute: &str) -> Option<Clock> {
    let hour = u8_at(hour).filter(|h| *h <= 24)?;
    let minute = if minute.is_empty() {
        None
    } else {
        Some(u8_at(minute).filter(|m| *m <= 59)?)
    };
    Some(Clock {
        hour,
        minute,
        second: None,
        meridiem: None,
        is_clock: false,
    })
}

fn time_range(_: &Romanian, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let separator = text_of(caps, 3);
    if separator.contains(['–', '-']) && separator.contains(char::is_whitespace) {
        return None;
    }
    let from = clock(text_of(caps, 1), text_of(caps, 2))?;
    let to = clock(text_of(caps, 4), text_of(caps, 5))?;
    Some(Match::new(whole, Token::TimeRange { from, to }))
}

/// `14:30`, `9:00`, and `ora 9` (the word is part of the span, so the
/// hour takes the feminine "ora două").
fn time(_: &Romanian, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let ora =
        group(caps, 1).filter(|r| !char_before(text, r.start).is_some_and(char::is_alphanumeric));
    if caps.get(3).is_none() && ora.is_none() {
        return None;
    }
    if let Some(minutes) = group(caps, 3) {
        if matches!(char_after(text, minutes.end), Some('.' | ','))
            && char_after(text, minutes.end + 1).is_some_and(|c| c.is_ascii_digit())
        {
            return None;
        }
    }
    let clock = clock(text_of(caps, 2), text_of(caps, 3))?;
    let start = ora.map_or(group(caps, 2)?.start, |r| r.start);
    Some(Match::new(start..whole.end, Token::Time(clock)))
}

fn date(r: &Romanian, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) || matches!(char_before(text, whole.start), Some('-' | '–')) {
        return None;
    }
    let (day, month, year) = if caps.get(1).is_some() {
        (
            Some(u8_at(text_of(caps, 1))?),
            u8_at(text_of(caps, 2))?,
            caps.get(3).map(|_| text_of(caps, 3)),
        )
    } else if caps.get(4).is_some() {
        if !iso_bounded(text, caps, 6, &whole) {
            return None;
        }
        (
            optional_u8(caps, 6)?,
            u8_at(text_of(caps, 5))?,
            Some(text_of(caps, 4)),
        )
    } else {
        (
            Some(u8_at(text_of(caps, 7))?),
            r.month(text_of(caps, 8))?,
            caps.get(9).map(|_| text_of(caps, 9)),
        )
    };
    if !day.is_none_or(|d| (1..=31).contains(&d)) || !(1..=12).contains(&month) {
        return None;
    }
    let year = match year {
        Some(y) => Some(y.parse::<u64>().ok()?),
        None => None,
    };
    Some(Match::new(whole, Token::Date { day, month, year }))
}

fn roman_number(s: &str) -> Option<u64> {
    let value = |c: char| -> Option<u64> {
        Some(match c {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => return None,
        })
    };
    let digits: Vec<u64> = s.chars().map(value).collect::<Option<_>>()?;
    let mut total = 0;
    for (i, d) in digits.iter().enumerate() {
        if digits.get(i + 1).is_some_and(|next| next > d) {
            total -= *d as i64;
        } else {
            total += *d as i64;
        }
    }
    u64::try_from(total).ok().filter(|n| *n > 0)
}

/// The `-lea`/`-a` suffix carries the gender; the article, when written,
/// joins the span so the spoken form starts with its own `al`/`a`.
fn ordinal(_: &Romanian, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let article =
        group(caps, 1).filter(|r| !char_before(text, r.start).is_some_and(char::is_alphanumeric));
    let number = group(caps, 2)?;
    let glued =
        char_after(text, whole.end).is_some_and(|c| c.is_alphanumeric() && !common::is_script(c));
    let before = char_before(text, number.start);
    if glued || (article.is_none() && before.is_some_and(|c| c.is_alphabetic() || c == '-')) {
        return None;
    }
    let written = &text[number.clone()];
    let n = if written.starts_with(|c: char| c.is_ascii_digit()) {
        if !digit_bounded(text, &number) {
            return None;
        }
        written.parse::<u64>().ok()?
    } else {
        roman_number(written)?
    };
    let gender = if text_of(caps, 3) == "lea" {
        Gender::Masculine
    } else {
        Gender::Feminine
    };
    let start = article.map_or(number.start, |r| r.start);
    let mut m = Match::new(start..whole.end, Token::Ordinal(n));
    m.agreement.gender = Some(gender);
    Some(m)
}
