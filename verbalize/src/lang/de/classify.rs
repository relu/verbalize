//! German recognisers in priority order.
//! The language-neutral ones come from [`common`]; times, dates, ordinals
//! and the ordinal range need German shapes and the ordinal trigger words.

use regex::Regex;

use super::lexicon::*;
use super::German;
use crate::lang::common::{
    self, group, is_trigger_bounded, iso_bounded, recognizer, text_of, Patterns,
};
use crate::lang::{
    char_after, char_before, digit_bounded, next_word, previous_word, Context, HSPACE,
};
use crate::pipeline::{Caps, Match, Recognizer};
use crate::token::*;

pub(super) fn recognizers(months: &str, abbreviations: Option<Regex>) -> Vec<Recognizer<German>> {
    let p = Patterns::new(&super::NUMERALS, &TABLES);
    let num = super::NUMERALS.num();
    let plain = super::NUMERALS.plain();
    let mut list = vec![
        // `@`, `://`, `www.` and a known TLD are unambiguous: above everything.
        recognizer(&p.electronic, common::electronic),
        recognizer(
            &format!(
                r"([0-9]{{1,2}})(?:([:.])([0-9]{{2}}))?({HSPACE}*[–-]{HSPACE}*|{HSPACE}+bis{HSPACE}+)([0-9]{{1,2}})(?:([:.])([0-9]{{2}}))?({HSPACE}*Uhr)?"
            ),
            time_range,
        ),
        recognizer(
            &format!(r"([0-9]{{1,2}})([:.])([0-9]{{2}})(?::([0-9]{{2}}))?({HSPACE}*Uhr)?"),
            time,
        ),
        // `um 1 Uhr`: an hour alone, read like `1:00 Uhr`.
        recognizer(&format!(r"([0-9]{{1,2}}){HSPACE}*Uhr"), hour),
        recognizer(
            &format!(
                r"(?:((?i:{TRIGGERS}))\s+)?(?:([0-9]{{1,2}})\.([0-9]{{1,2}})(?:(\.)([0-9]{{4}})?)?|([0-9]{{4}})-([0-9]{{2}})(?:-([0-9]{{2}}))?|([0-9]{{1,2}})\.{HSPACE}+({months})(?:{HSPACE}+([0-9]{{4}}))?)"
            ),
            date,
        ),
        recognizer(&p.iban, common::iban),
        recognizer(&p.card, common::digit_groups),
        recognizer(
            r"(\+[0-9]{1,3}|\(0[0-9]+\)|0[0-9]+)((?:[ /\-][0-9]+)+)",
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
        recognizer(
            &format!(
                r"(?:(€|\$|£|EUR|USD|GBP|CHF){HSPACE}*)?(-?{num})(?:{HSPACE}*(Mio\.|Mio|(?i:Millionen|Million|Milliarden|Milliarde|Billionen|Billion|Tausend)|Mrd\.|Bio\.|Tsd\.))?(?:{HSPACE}*(€|EUR|Euro|\$|USD|Dollar|£|GBP|Pfund|CHF|Franken))?(?:/(\p{{L}}[\p{{L}}.]*))?"
            ),
            common::money,
        ),
        recognizer(&p.percent, common::percent),
        recognizer(&p.degrees, common::degrees),
        recognizer(
            &format!(r"(-?{plain}){HSPACE}*(?:[x×]|-mal)(/\p{{L}}+)?"),
            common::repetition,
        ),
        recognizer(r"([0-9]+)\.[–-]([0-9]+)\.", ordinal_range),
        recognizer(&p.range, common::range),
        recognizer(&p.measure, common::measure),
        recognizer(&p.hyphenated_measure, common::hyphenated_measure),
        recognizer(
            &format!(
                r"(?:((?i:{TRIGGERS}|{INDEFINITE_ARTICLES}))\s+)?([0-9]+)\.(?:{HSPACE}+(\p{{L}}+))?"
            ),
            ordinal,
        ),
        // Below Range and Ordinal, above Year and Decimal: `2.3` and `3.10`
        // are section or version numbers once the dated shapes have passed.
        recognizer(&p.dotted, common::dotted),
        // Slash fraction before Year: `1/1000` is a thousandth, not a year.
        recognizer(&p.slash_fraction, common::slash_fraction),
        recognizer(&p.year, common::year),
        recognizer(&p.decimal, common::decimal),
        recognizer(&p.mixed_fraction, common::mixed_fraction),
        recognizer(&p.score, common::score),
        recognizer(&p.cardinal_grouped, common::cardinal),
        // Plain runs last: a grouped candidate rejected above never loses a run.
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

/// The declension and case a trigger word imposes on an ordinal.
pub(super) fn ordinal_agreement(trigger: Option<&str>, gender: Option<Gender>) -> Agreement {
    let lower = trigger.map(str::to_lowercase);
    let (declension, case) = match lower.as_deref() {
        Some(t) if WEAK_DATIVE.contains(&t) => (Declension::Weak, Case::Dative),
        Some(t) if WEAK_ACCUSATIVE.contains(&t) => (Declension::Weak, Case::Accusative),
        Some(t) if WEAK_GENITIVE.contains(&t) => (Declension::Weak, Case::Genitive),
        Some(t) if WEAK_NOMINATIVE.contains(&t) => (Declension::Weak, Case::Nominative),
        Some("der") if gender == Some(Gender::Masculine) => (Declension::Weak, Case::Nominative),
        Some("der") => (Declension::Weak, Case::Dative),
        Some(t) if STRONG_DATIVE.contains(&t) => (Declension::Strong, Case::Dative),
        _ => (Declension::Strong, Case::Nominative),
    };
    Agreement {
        case,
        gender,
        declension,
        ..Agreement::default()
    }
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

fn time_range(_: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let separator = text_of(caps, 4);
    if separator.contains(['–', '-']) && separator.contains(char::is_whitespace) {
        return None;
    }
    let from = clock(text_of(caps, 1), text_of(caps, 3))?;
    let to = clock(text_of(caps, 5), text_of(caps, 7))?;
    let uhr = caps.get(8).is_some();
    let dotted = text_of(caps, 2) == "." || text_of(caps, 6) == ".";
    let with_minutes = from.minute.is_some() && to.minute.is_some();
    if with_minutes && text_of(caps, 2) != text_of(caps, 6) {
        return None;
    }
    let bare_hours = from.minute.is_none() && to.minute.is_none();
    if !(with_minutes || bare_hours) || (dotted && !uhr) || (bare_hours && !uhr) {
        return None;
    }
    Some(Match::new(whole, Token::TimeRange { from, to }))
}

fn time(_: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    if text_of(caps, 2) == "." && (caps.get(4).is_some() || caps.get(5).is_none()) {
        return None;
    }
    let digits_end = group(caps, 4).or_else(|| group(caps, 3))?.end;
    if matches!(crate::lang::char_after(text, digits_end), Some('.' | ','))
        && crate::lang::char_after(text, digits_end + 1).is_some_and(|c| c.is_ascii_digit())
    {
        return None;
    }
    let mut clock = clock(text_of(caps, 1), text_of(caps, 3))?;
    if caps.get(4).is_some() {
        clock.second = Some(u8_at(text_of(caps, 4)).filter(|s| *s <= 59)?);
    }
    Some(Match::new(whole, Token::Time(clock)))
}

/// An hour before "Uhr": `um 1 Uhr`. Not the tail of a longer number or
/// range (`1.5 Uhr`, `50-2 Uhr`) and not the start of a word (`Uhrzeit`).
fn hour(_: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let joined_to_number = {
        let mut before = text[..whole.start].chars().rev();
        matches!(before.next(), Some('.' | ',' | ':' | '-' | '–' | '/'))
            && before.next().is_some_and(|c| c.is_ascii_digit())
    };
    if !digit_bounded(text, &whole)
        || joined_to_number
        || char_after(text, whole.end).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let clock = clock(text_of(caps, 1), "")?;
    Some(Match::new(whole, Token::Time(clock)))
}

fn date(g: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let trigger = group(caps, 1)
        .filter(|r| is_trigger_bounded(text, r))
        .map(|r| &text[r]);
    let start = caps.first_start(2)?;
    let whole = start..group(caps, 0)?.end;
    // `1.–3. November` is a Range of ordinals, not a date after a dash.
    if !digit_bounded(text, &whole) || matches!(char_before(text, start), Some('-' | '–')) {
        return None;
    }
    let (day, month, year) = if caps.get(2).is_some() {
        let (day, month) = (text_of(caps, 2), text_of(caps, 3));
        // Without the trailing period only the zero-padded form (`02.03`)
        // is a date; `3.10` is a section or version number.
        if caps.get(4).is_none() && !(day.starts_with('0') || month.starts_with('0')) {
            return None;
        }
        (
            Some(u8_at(day)?),
            u8_at(month)?,
            caps.get(5).map(|_| text_of(caps, 5)),
        )
    } else if caps.get(6).is_some() {
        if !iso_bounded(text, caps, 8, &whole) {
            return None;
        }
        (
            optional_u8(caps, 8)?,
            u8_at(text_of(caps, 7))?,
            Some(text_of(caps, 6)),
        )
    } else {
        (
            Some(u8_at(text_of(caps, 9))?),
            g.month(text_of(caps, 10))?,
            caps.get(11).map(|_| text_of(caps, 11)),
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
    m.agreement = ordinal_agreement(trigger, Some(Gender::Masculine));
    Some(m)
}

/// `1.–3. November`: both ends ordinals with the same agreement.
fn ordinal_range(g: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole)
        || char_before(text, whole.start).is_some_and(|c| c == '-' || c == '–')
    {
        return None;
    }
    let (word, _) = next_word(text, whole.end)?;
    let gender = g.ordinal_noun(word)?;
    let end =
        |i: usize| -> Option<RangeEnd> { Some(RangeEnd::Ordinal(text_of(caps, i).parse().ok()?)) };
    let mut m = Match::new(
        whole.clone(),
        Token::Range {
            from: end(1)?,
            to: end(2)?,
            unit: None,
        },
    );
    m.agreement = ordinal_agreement(previous_word(text, whole.start), Some(gender));
    Some(m)
}

fn ordinal(g: &German, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let trigger = group(caps, 1)
        .filter(|r| is_trigger_bounded(text, r))
        .map(|r| &text[r]);
    let number = group(caps, 2)?;
    if !digit_bounded(text, &number)
        || char_before(text, number.start).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let next = caps.get(3).map(|_| text_of(caps, 3));
    let gender = next.and_then(|w| g.ordinal_noun(w));
    // Determiner, `N.`, lower-case adjective, noun: the ordinal inflects like
    // the adjective (Duden: successive attributive adjectives take one
    // ending). Without a determiner `N.` is a list number or a sentence end;
    // without a capitalised noun after it the word is no attribute
    // (`bis 3. kommen`, `nach 2. kontrolle in`).
    let adjective = match (trigger, group(caps, 3)) {
        (Some(_), Some(word)) if gender.is_none() && before_noun(text, word.end) => {
            adjective_ending(&text[word])
        }
        _ => None,
    };
    // Otherwise only a trigger whose ending does not depend on the noun's
    // gender resolves the ordinal without a lexicon noun (`am 5.`,
    // `am 100. Geburtstag`).
    if adjective.is_none()
        && gender.is_none()
        && (!trigger.is_some_and(gender_free_trigger)
            || crate::lang::char_after(text, number.end + 1).is_some_and(char::is_alphanumeric))
    {
        return None;
    }
    let n: u64 = text[number.clone()].parse().ok()?;
    let mut m = Match::new(number.start..number.end + 1, Token::Ordinal(n));
    m.agreement = match adjective {
        Some(ending) => ending_agreement(ending),
        None => ordinal_agreement(trigger, gender),
    };
    Some(m)
}

/// The words after `at` are adjectives up to a capitalised noun
/// (`große Sieg`, `große rote Ball`).
fn before_noun(text: &str, mut at: usize) -> bool {
    while let Some((word, range)) = next_word(text, at) {
        if word.starts_with(char::is_uppercase) {
            return true;
        }
        if adjective_ending(word).is_none() {
            return false;
        }
        at = range.end;
    }
    false
}

/// The inflection ending (`e`, `en`, `em`, `er`, `es`) of a lower-case word
/// that reads as an attributive adjective.
fn adjective_ending(word: &str) -> Option<&'static str> {
    if !word.starts_with(char::is_lowercase) || word.chars().count() < 4 {
        return None;
    }
    const ENDINGS: [&str; 5] = ["en", "em", "er", "es", "e"];
    let ending = ENDINGS.into_iter().find(|ending| word.ends_with(ending))?;
    let determiner = DETERMINER_STEMS.iter().any(|stem| {
        word.strip_prefix(stem)
            .is_some_and(|rest| rest.is_empty() || ENDINGS.contains(&rest))
    });
    (!determiner && !NOT_ADJECTIVES.contains(&word)).then_some(ending)
}

/// An agreement whose ordinal ruleset ends in `ending`: the ruleset is the
/// only thing the agreement selects, so the ending identifies it.
fn ending_agreement(ending: &str) -> Agreement {
    let (declension, case, gender) = match ending {
        "e" => (Declension::Weak, Case::Nominative, None),
        "en" => (Declension::Weak, Case::Dative, None),
        "em" => (Declension::Strong, Case::Dative, Some(Gender::Masculine)),
        "er" => (
            Declension::Strong,
            Case::Nominative,
            Some(Gender::Masculine),
        ),
        _ => (Declension::Strong, Case::Nominative, Some(Gender::Neuter)),
    };
    Agreement {
        case,
        gender,
        declension,
        ..Agreement::default()
    }
}

/// Triggers that fix the ordinal's ending for every gender (weak
/// dative/accusative/genitive `-en`, weak nominative after `die`/`das` `-e`).
fn gender_free_trigger(trigger: &str) -> bool {
    let t = trigger.to_lowercase();
    let t = t.as_str();
    WEAK_DATIVE.contains(&t)
        || WEAK_ACCUSATIVE.contains(&t)
        || WEAK_GENITIVE.contains(&t)
        || WEAK_NOMINATIVE.contains(&t)
}

impl German {
    /// Gender of the noun an ordinal precedes: a month (masculine) or a
    /// lexicon noun; `None` means the context is not resolvable.
    fn ordinal_noun(&self, word: &str) -> Option<Gender> {
        if self.month(word).is_some() {
            return Some(Gender::Masculine);
        }
        self.lexicon_noun(word)
    }

    pub(super) fn lexicon_noun(&self, word: &str) -> Option<Gender> {
        ORDINAL_NOUNS
            .iter()
            .find(|(noun, _)| {
                word.strip_prefix(noun)
                    .is_some_and(|rest| NOUN_ENDINGS.contains(&rest))
            })
            .map(|(_, g)| *g)
    }
}
