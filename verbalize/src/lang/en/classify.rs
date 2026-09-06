//! English recognisers in priority order. The language-neutral ones come
//! from [`common`]; times (12- and 24-hour), dates in both regional
//! orders, `1st`-style ordinals and the telephone/money shapes are English.

use regex::Regex;

use super::lexicon::TABLES;
use super::English;
use crate::lang::common::{self, group, iso_bounded, recognizer, text_of, Patterns};
use crate::lang::{char_after, char_before, digit_bounded, Context};
use crate::pipeline::{Caps, Match, Recognizer};
use crate::token::*;

/// `am`, `pm`, `a.m.`, `P.M.` — never `pm.` with a sentence-final period.
const MERIDIEM: &str = r"(?i:[ap]\.m\.|[ap]m)";

pub(super) fn recognizers(months: &str, abbreviations: Option<Regex>) -> Vec<Recognizer<English>> {
    let p = Patterns::new(&super::NUMERALS, &TABLES);
    let num = super::NUMERALS.num();
    let plain = super::NUMERALS.plain();
    let mut list = vec![
        // `@`, `://`, `www.` and a known TLD are unambiguous: above everything.
        recognizer(&p.electronic, common::electronic),
        recognizer(
            &format!(
                r"([0-9]{{1,2}})(?::([0-9]{{2}}))?(?:\s*({MERIDIEM}))?(\s*[–-]\s*|\s+to\s+)([0-9]{{1,2}})(?::([0-9]{{2}}))?(?:\s*({MERIDIEM}))?"
            ),
            time_range,
        ),
        recognizer(
            &format!(r"([0-9]{{1,2}})(?::([0-9]{{2}})(?::([0-9]{{2}}))?)?(?:\s*({MERIDIEM}))?"),
            time,
        ),
        recognizer(
            &format!(
                r"(?:([0-9]{{1,2}})/([0-9]{{1,2}})/([0-9]{{4}})|([0-9]{{4}})-([0-9]{{2}})(?:-([0-9]{{2}}))?|([0-9]{{4}})/([0-9]{{1,2}})/([0-9]{{1,2}})|({months})\.?\s+([0-9]{{1,2}})(?:st|nd|rd|th)?|([0-9]{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?({months})\.?(?:\s+([0-9]{{4}}))?)"
            ),
            date,
        ),
        recognizer(&p.iban, common::iban),
        // Fixed shapes before Telephone: a 16-digit card, a US SSN.
        recognizer(&p.card, common::digit_groups),
        recognizer(r"[0-9]{3}-[0-9]{2}-[0-9]{4}", common::digit_groups),
        recognizer(
            r"(\+[0-9]{1,3}|\(?[0-9]{3}\)?|0[0-9]+)((?:[ /.\-]\(?[0-9]+\)?)+)",
            common::telephone,
        ),
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
        // `1st` is unambiguous, and `st`/`nd`/`rd` are also unit symbols
        // (stone, nano-day, rod), so ordinals outrank Measure here.
        recognizer(
            r"([0-9]+)(?:st|nd|rd|th)[–-]([0-9]+)(?:st|nd|rd|th)",
            ordinal_range,
        ),
        // `1/4th`: the ordinal suffix joins the span.
        recognizer(r"([0-9]+)/([0-9]+)(?:st|nd|rd|th)", common::slash_fraction),
        recognizer(r"([0-9]+)(st|nd|rd|th)", ordinal),
        recognizer(
            &format!(
                r"(?:(US\$|\$|£|€)\s?)?(-?(?:{num}|\.[0-9]+))(?:\s*(thousand|million|billion|trillion|bn|k|M))?(?:\s*(USD|EUR|GBP|CHF|dollars?|euros?|pounds?|francs?))?(?:/(\p{{L}}[\p{{L}}.]*))?"
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
        // Above Decimal so `1.2.7` is not "one point two" plus a period.
        recognizer(&p.dotted, common::dotted),
        // Slash fraction before Year: `1/1000` is a thousandth, not a year.
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

fn meridiem(text: &str) -> Option<Meridiem> {
    match text.chars().next()?.to_ascii_lowercase() {
        'a' => Some(Meridiem::Am),
        'p' => Some(Meridiem::Pm),
        _ => None,
    }
}

fn clock(hour: &str, minute: &str, meridiem: Option<Meridiem>) -> Option<Clock> {
    let hour = u8_at(hour)?;
    let limit = if meridiem.is_some() { 12 } else { 24 };
    if hour > limit {
        return None;
    }
    let minute = if minute.is_empty() {
        None
    } else {
        Some(u8_at(minute).filter(|m| *m <= 59)?)
    };
    Some(Clock {
        hour,
        minute,
        second: None,
        meridiem,
        is_clock: false,
    })
}

/// The meridiem must not be glued to a following letter or script (`2 pmol`).
fn meridiem_bounded(text: &str, caps: &Caps<'_>, i: usize) -> bool {
    caps.get(i).is_none_or(|m| {
        !char_after(text, m.end).is_some_and(|c| c.is_alphanumeric() || common::is_script(c))
    })
}

/// A short all-caps word after the time (`EST`, `UTC`, `CET`) reads as a
/// time zone abbreviation, making an `hh:mm:ss` reading unambiguously a
/// clock rather than a duration.
fn zone_follows(text: &str, at: usize) -> bool {
    let rest = text[at..].trim_start();
    let letters = rest.chars().take_while(char::is_ascii_uppercase).count();
    (2..=6).contains(&letters)
        && rest
            .chars()
            .nth(letters)
            .is_none_or(|c| !(c.is_alphanumeric() || common::is_script(c)))
}

fn time_range(_: &English, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole)
        || !meridiem_bounded(text, caps, 3)
        || !meridiem_bounded(text, caps, 7)
    {
        return None;
    }
    let separator = text_of(caps, 4);
    if separator.contains(['–', '-']) && separator.contains(char::is_whitespace) {
        return None;
    }
    let to_meridiem = meridiem(text_of(caps, 7));
    let from_meridiem = meridiem(text_of(caps, 3)).or(to_meridiem);
    let from = clock(text_of(caps, 1), text_of(caps, 2), from_meridiem)?;
    let to = clock(text_of(caps, 5), text_of(caps, 6), to_meridiem)?;
    let with_minutes = from.minute.is_some() && to.minute.is_some();
    let bare_hours = from.minute.is_none() && to.minute.is_none();
    // Bare hours are a time range only with am/pm (`2–4 pm`), else a Range.
    if !(with_minutes || bare_hours) || (bare_hours && to_meridiem.is_none()) {
        return None;
    }
    Some(Match::new(whole, Token::TimeRange { from, to }))
}

fn time(_: &English, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) || !meridiem_bounded(text, caps, 4) {
        return None;
    }
    let meridiem = meridiem(text_of(caps, 4));
    if caps.get(2).is_none() && meridiem.is_none() {
        return None;
    }
    if let Some(minutes) = group(caps, 2) {
        if matches!(char_after(text, minutes.end), Some('.' | ','))
            && char_after(text, minutes.end + 1).is_some_and(|c| c.is_ascii_digit())
        {
            return None;
        }
    }
    let mut clock = clock(text_of(caps, 1), text_of(caps, 2), meridiem)?;
    if caps.get(3).is_some() {
        clock.second = Some(u8_at(text_of(caps, 3)).filter(|s| *s <= 59)?);
        clock.is_clock = meridiem.is_some() || zone_follows(text, whole.end);
    }
    Some(Match::new(whole, Token::Time(clock)))
}

/// Numeric and ISO dates follow the region (month-day-year in en-US,
/// day-month-year in en-GB); month-name forms keep their written order,
/// recorded as the month's position: `NounPosition::Before` for
/// `November 1`, `After` for `1 November`.
fn date(e: &English, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) || matches!(char_before(text, whole.start), Some('-' | '–')) {
        return None;
    }
    let mut noun = NounPosition::Absent;
    let (day, month, year) = if caps.get(1).is_some() {
        let (a, b) = (u8_at(text_of(caps, 1))?, u8_at(text_of(caps, 2))?);
        let (day, month) = if e.day_first { (a, b) } else { (b, a) };
        (Some(day), month, Some(text_of(caps, 3)))
    } else if caps.get(4).is_some() {
        if !iso_bounded(text, caps, 6, &whole) {
            return None;
        }
        (
            optional_u8(caps, 6)?,
            u8_at(text_of(caps, 5))?,
            Some(text_of(caps, 4)),
        )
    } else if caps.get(7).is_some() {
        // `2016/07/03`: year first is unambiguous in either region.
        (
            Some(u8_at(text_of(caps, 9))?),
            u8_at(text_of(caps, 8))?,
            Some(text_of(caps, 7)),
        )
    } else if caps.get(10).is_some() {
        noun = NounPosition::Before;
        (
            Some(u8_at(text_of(caps, 11))?),
            e.month(text_of(caps, 10))?,
            None,
        )
    } else {
        noun = NounPosition::After;
        (
            Some(u8_at(text_of(caps, 12))?),
            e.month(text_of(caps, 13))?,
            caps.get(14).map(|_| text_of(caps, 14)),
        )
    };
    if !day.is_none_or(|d| (1..=31).contains(&d)) || !(1..=12).contains(&month) {
        return None;
    }
    let year = match year {
        Some(y) => Some(y.parse::<u64>().ok()?),
        None => None,
    };
    let mut m = Match::new(whole, Token::Date { day, month, year });
    m.agreement.noun = noun;
    Some(m)
}

fn ordinal_range(_: &English, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole)
        || char_after(text, whole.end).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let end =
        |i: usize| -> Option<RangeEnd> { Some(RangeEnd::Ordinal(text_of(caps, i).parse().ok()?)) };
    Some(Match::new(
        whole,
        Token::Range {
            from: end(1)?,
            to: end(2)?,
        },
    ))
}

fn ordinal(_: &English, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let number = group(caps, 1)?;
    if !digit_bounded(text, &number)
        || char_before(text, number.start).is_some_and(char::is_alphanumeric)
        || char_after(text, whole.end).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let n: u64 = text[number].parse().ok()?;
    Some(Match::new(whole, Token::Ordinal(n)))
}
