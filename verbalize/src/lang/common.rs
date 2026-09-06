//! Recognisers and spoken-form helpers that are the same in every
//! language once the number format, the unit tables and a few word lists
//! are supplied through [`Context`]. A language module composes its
//! priority list from these and its own (dates, times, ordinals).

use std::ops::Range;

use regex::Regex;

use super::{
    char_after, char_before, digit_bounded, letter_free, next_word, units::Units, Context,
    Numerals, Tables,
};
use crate::pipeline::{word_bounded, Caps, Match, Recognizer};
use crate::token::*;
use crate::{spell, Language};

pub(crate) const SUPERSCRIPT: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
pub(crate) const SUBSCRIPT: &str = "₀₁₂₃₄₅₆₇₈₉";
pub(crate) const UNICODE_FRACTION_CHARS: &str = "½⅓⅔¼¾⅕⅛⅜⅝⅞";

/// A unit symbol: letters (incl. µ, Ω), superscripts, an abbreviation dot.
const SYM: &str = r"(?:I\. ?E\.|\p{L}[\p{L}²³.]*)";

/// Top-level domains a bare `name.tld` is recognised with; `z.B.`, `Dr.`
/// and `3.10.` never end in one of these. A language removes the ones
/// that are words of its own (`Tables::tld_words`).
const TLDS: &[&str] = &[
    "com", "de", "org", "net", "ro", "eu", "io", "info", "edu", "gov", "uk", "at", "ch", "fr",
    "it", "es", "nl", "me", "tv", "co", "biz", "dev", "app", "html", "htm", "php", "pdf", "jpg",
    "jpeg", "png",
];

/// Email, `scheme://…`, `www.…`, `name.tld[/path]`, IPv4, `@handle`.
fn electronic_pattern(tables: &Tables) -> String {
    let tlds: Vec<&str> = TLDS
        .iter()
        .copied()
        .filter(|tld| !tables.tld_words.contains(tld))
        .collect();
    concat!(
        r#"[\p{L}0-9][\p{L}0-9.+_\-]*@[\p{L}0-9\-]+(?:\.[\p{L}0-9\-]+)+(?:/[^\s<>"']*)?"#,
        r#"|(?:https?|ftp|file)://[^\s<>"']+"#,
        r#"|www\.[^\s<>"']+"#,
        r"|[\p{L}0-9\-]+(?:\.[\p{L}0-9\-]+)*\.(?:TLDS)",
        r#"(?:/[^\s<>"']*)?"#,
        r"|[0-9]{1,3}(?:\.[0-9]{1,3}){3}",
        r"|@[\p{L}0-9_]+(?:\.[\p{L}0-9_]+)*",
    )
    .replace("TLDS", &tlds.join("|"))
}

pub(crate) fn recognizer<C>(
    pattern: &str,
    parse: fn(&C, &str, &Caps<'_>) -> Option<Match>,
) -> Recognizer<C> {
    Recognizer {
        regex: Regex::new(pattern).expect("hand-written pattern"),
        parse,
    }
}

/// The regex patterns of the shared recognisers for one number format.
pub(crate) struct Patterns {
    pub electronic: String,
    pub scientific: String,
    pub chemical: String,
    pub power: String,
    pub zone_offset: String,
    pub math: String,
    pub angle: String,
    pub blood_pressure: String,
    pub range_rate: String,
    pub ratio: String,
    pub dose: String,
    pub percent: String,
    pub degrees: String,
    pub measure: String,
    pub hyphenated_measure: String,
    pub range: String,
    pub year: String,
    pub dotted: String,
    pub decimal: String,
    pub slash_fraction: String,
    pub mixed_fraction: String,
    pub score: String,
    pub cardinal_grouped: String,
    pub cardinal_plain: String,
    pub iban: String,
    pub card: String,
    pub unicode_fraction: String,
    pub roman: String,
    pub paragraph: String,
}

impl Patterns {
    pub(crate) fn new(numerals: &Numerals, tables: &Tables) -> Self {
        let int = numerals.int();
        let num = numerals.num();
        let plain = numerals.plain();
        let dec = regex::escape(&numerals.decimal.to_string());
        let seg = format!(r"(?:{plain}\s?)?\p{{L}}[\p{{L}}²³.]*");
        let unit = format!("(?:{SYM}(?:/{seg})*|(?:/{seg})+)");
        let compass: String = tables.compass.iter().map(|(c, _)| *c).collect();
        Patterns {
            electronic: electronic_pattern(tables),
            scientific: format!(
                r"(-?[0-9]+(?:[.,][0-9]+)?)\s*(?:[×·x*]\s*10(?:\^(-?[0-9]+)|([{SUPERSCRIPT}⁻⁺]+))|[eE]([+-]?[0-9]+))(?:\s*(\p{{L}}[\p{{L}}²³⁻¹.]*(?:/{seg})*))?"
            ),
            chemical: format!(r"(?:[A-Z][a-z]?[{SUBSCRIPT}]*)+(?:[{SUPERSCRIPT}]*[⁺⁻])?"),
            power: format!(r"(?:(-?{plain})|(\p{{L}}))(?:([{SUPERSCRIPT}⁻]+)|\^(-?[0-9]+))"),
            zone_offset: r"(UTC|GMT)([+\-−])([0-9]{1,2})".into(),
            math: format!(
                r"(?:(?:-?{plain}|[\p{{L}}∞π])\s*)?(?:[+×·*÷=≠<>≤≥≈±√]|\s-\s|−)\s*(?:-?{plain}|[\p{{L}}∞π])(?:\s*(?:[+×·*÷=≠<>≤≥≈±]|\s-\s|−)\s*(?:-?{plain}|[\p{{L}}∞π]))*|∞"
            ),
            angle: format!(
                r#"(-?{plain})\s*°(?:\s*([0-9]{{1,2}})\s*[′'])?(?:\s*([0-9]{{1,2}})\s*[″"])?(?:\s*([{compass}]))?"#
            ),
            blood_pressure: format!(
                r"(?:({})\s+)?([0-9]+)/([0-9]+)(?:\s*(mmHg))?",
                tables.blood_pressure.join("|")
            ),
            range_rate: r"([0-9]+)-([0-9]+)/(\p{L}+)".into(),
            ratio: format!(
                r"(?:({})\s+)?({int})(\s*):(\s*)({int})",
                tables.ratio_triggers.join("|")
            ),
            dose: format!(r"(?:[0-9]{{1,3}}|½)(?:-(?:[0-9]{{1,3}}|½)){{2,3}}(?:\s*({unit}))?"),
            percent: format!(r"(-?{num})\s*([%‰])"),
            degrees: format!(r"(-?{num})\s*°\s?([CF])?"),
            measure: format!(r"(-?{num})(\s?)({unit})"),
            hyphenated_measure: r"([0-9]+)-(\p{L}{1,4})[-\s](\p{L})".into(),
            range: format!(r"({int})[–-]({int})"),
            year: format!(r"([0-9]{{4}})({})?", regex::escape(tables.decade_suffix)),
            decimal: format!(r"(-?{int}){dec}([0-9]+)"),
            dotted: r"[0-9]+(?:\.[0-9]+)+".into(),
            slash_fraction: r"([0-9]+)/([0-9]+)".into(),
            mixed_fraction: format!(r"([0-9]+) ?([{UNICODE_FRACTION_CHARS}])"),
            score: r"([0-9]{1,2}):([0-9]{1,2})".into(),
            cardinal_grouped: format!(r"(-?{int})"),
            cardinal_plain: r"(-?[0-9]+)".into(),
            iban: r"([A-Z]{2})([0-9]{2}(?: [A-Z0-9]{4}){2,}(?: [A-Z0-9]{1,4})?)".into(),
            card: r"[0-9]{4}(?:[ \u{a0}-][0-9]{4}){3}".into(),
            unicode_fraction: format!(r"[{UNICODE_FRACTION_CHARS}]"),
            roman: r"(IV|I{1,3})(?:-(IV|I{1,3}))?".into(),
            paragraph: r"§§?\s*".into(),
        }
    }
}

// -------------------------------------------------------------- helpers

pub(crate) fn group(caps: &Caps<'_>, i: usize) -> Option<Range<usize>> {
    caps.get(i)
}

pub(crate) fn text_of<'a>(caps: &Caps<'a>, i: usize) -> &'a str {
    caps.text(i)
}

/// A leading `-` counts as a sign only when nothing alphanumeric precedes it.
pub(crate) fn signed(text: &str, range: &Range<usize>) -> (bool, Range<usize>) {
    if text[range.clone()].starts_with(['-', '−']) {
        let sign_ok =
            !char_before(text, range.start).is_some_and(|c| c.is_alphanumeric() || c == '-');
        if !sign_ok {
            let skip = text[range.clone()].chars().next().map_or(1, char::len_utf8);
            return (false, range.start + skip..range.end);
        }
        return (true, range.clone());
    }
    (false, range.clone())
}

/// The numeral at `range` (sign handled, digit-bounded) with its final range.
pub(crate) fn numeral_at(
    numerals: &Numerals,
    text: &str,
    range: &Range<usize>,
) -> Option<(Numeral, Range<usize>)> {
    let (negative, range) = signed(text, range);
    if !digit_bounded(text, &range) {
        return None;
    }
    let mut n = numerals.parse(text[range.clone()].trim_start_matches(['-', '−']))?;
    n.negative = negative;
    Some((n, range))
}

pub(crate) fn is_trigger_bounded(text: &str, range: &Range<usize>) -> bool {
    !char_before(text, range.start).is_some_and(char::is_alphanumeric)
}

/// An ISO date whose day group `day` did not participate is the
/// year-month form (`2003-03`); it must not continue into another `-digit`
/// group, which would be some other dashed number.
pub(crate) fn iso_bounded(text: &str, caps: &Caps<'_>, day: usize, whole: &Range<usize>) -> bool {
    caps.get(day).is_some()
        || char_after(text, whole.end) != Some('-')
        || !char_after(text, whole.end + 1).is_some_and(|ch| ch.is_ascii_digit())
}

/// A bare `G` numerator (`G`, `G/l`), which a language reads as Giga only
/// in its own contexts; `Gpt`, `GB` and other `G…` symbols are not bare.
pub(crate) fn bare_giga(text: &str) -> bool {
    text.strip_prefix('G')
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

pub(crate) fn superscript_digit(c: char) -> Option<u32> {
    SUPERSCRIPT.chars().position(|d| d == c).map(|at| at as u32)
}

pub(crate) fn subscript_digit(c: char) -> Option<u32> {
    SUBSCRIPT.chars().position(|d| d == c).map(|at| at as u32)
}

pub(crate) fn is_script(c: char) -> bool {
    subscript_digit(c).is_some() || superscript_digit(c).is_some() || "⁺⁻".contains(c)
}

fn superscript_int(s: &str) -> Option<i32> {
    let mut value: i32 = 0;
    let mut negative = false;
    for c in s.chars() {
        match c {
            '⁻' => negative = true,
            '⁺' => {}
            c => {
                value = value
                    .checked_mul(10)?
                    .checked_add(superscript_digit(c)? as i32)?
            }
        }
    }
    Some(if negative { -value } else { value })
}

pub(crate) fn roman_value(s: &str) -> Option<u8> {
    Some(match s {
        "I" => 1,
        "II" => 2,
        "III" => 3,
        "IV" => 4,
        _ => return None,
    })
}

/// Default [`Context::unit`]: `X⁻¹` is a bare rate, else a compound.
pub(crate) fn unit(units: &Units, numerals: &Numerals, text: &str, bare: bool) -> Option<Unit> {
    if let Some(base) = text.strip_suffix("⁻¹") {
        let mut unit = units.resolve_compound(numerals, base, false)?;
        let numerator = unit.numerator.take()?;
        unit.denominators.insert(0, numerator);
        return Some(unit);
    }
    units.resolve_compound(numerals, text, bare)
}

/// A unit right after `at`, with the byte offset it ends at, for classes
/// that own a trailing unit (§7.6).
pub(crate) fn trailing_unit<C: Context>(c: &C, text: &str, at: usize) -> Option<(Unit, usize)> {
    let rest = &text[at..];
    let start = rest.len() - rest.trim_start().len();
    if start > 1 {
        return None;
    }
    let symbol_len = rest[start..]
        .find(|ch: char| !(ch.is_alphabetic() || "²³⁻¹./,0123456789 ".contains(ch)))
        .unwrap_or(rest.len() - start);
    let mut symbol = rest[start..start + symbol_len].trim_end_matches([' ', ',']);
    if let Some(stripped) = symbol.strip_suffix('.') {
        if c.unit(symbol, true).is_none() {
            symbol = stripped;
        }
    }
    let unit = c.unit(symbol, true)?;
    let end = at + start + symbol.len();
    if char_after(text, end).is_some_and(|ch| ch.is_alphanumeric() || is_script(ch)) {
        return None;
    }
    Some((unit, end))
}

// ----------------------------------------------------------- recognisers

/// `(+CC|0…)` followed by separated digit groups; the language supplies
/// the shape regex (groups 1 = first block, 2 = the rest).
pub(crate) fn telephone<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole)
        || char_before(text, whole.start).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let first = text_of(caps, 1);
    let international = first.starts_with('+');
    let digits_of = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    let mut groups = vec![digits_of(first)];
    groups.extend(
        text_of(caps, 2)
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    );
    let digits: usize = groups.iter().map(String::len).sum();
    if digits < 7 || groups.len() < 2 {
        return None;
    }
    Some(Match::new(
        whole,
        Token::Telephone {
            groups,
            international,
        },
    ))
}

/// Separated digit groups of a fixed shape (`4111 1111 1111 1111`,
/// `123-45-6789`): digit by digit, a pause between groups.
pub(crate) fn digit_groups<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let groups = text[whole.clone()]
        .split([' ', '\u{a0}', '-'])
        .map(str::to_string)
        .collect();
    Some(Match::new(whole, Token::DigitGroups(groups)))
}

/// Emails, URLs, `www.` and bare domains (a known TLD), IPv4 addresses and
/// `@handles`, kept as written; a sentence-final period or bracket after
/// the shape is not part of it.
pub(crate) fn electronic<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let mut whole = group(caps, 0)?;
    while text[whole.clone()].ends_with(['.', ',', ';', ':', ')', '!', '?']) {
        whole.end -= 1;
    }
    let written = &text[whole.clone()];
    if written.is_empty()
        || char_before(text, whole.start)
            .is_some_and(|ch| ch.is_alphanumeric() || "@./".contains(ch))
        || char_after(text, whole.end).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    // A bare domain or handle needs a letter somewhere; digits with dots is an IPv4.
    let dotted_digits = written.chars().all(|ch| ch.is_ascii_digit() || ch == '.');
    if dotted_digits && written.split('.').count() != 4 {
        return None;
    }
    Some(Match::new(whole, Token::Electronic(written.to_string())))
}

pub(crate) fn iban<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let letters = group(caps, 1)?;
    let digits = group(caps, 2)?;
    if !digit_bounded(text, &digits)
        || char_before(text, letters.start).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    // Bank-code letters inside the groups (`GB29 NWBK …`) are read one by one too.
    let run: String = text[digits.clone()]
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    Some(Match::new(digits, Token::Digits(run)))
}

pub(crate) fn scientific<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let mantissa_range = group(caps, 1)?;
    let (negative, mantissa_range) = signed(text, &mantissa_range);
    if !digit_bounded(text, &mantissa_range) {
        return None;
    }
    // Scientific notation accepts either decimal mark (`1.5E+08`, `1,5e-6`).
    let written = text[mantissa_range.clone()]
        .trim_start_matches('-')
        .replace(',', ".");
    let dot = Numerals {
        groups: &[],
        decimal: '.',
    };
    let mut mantissa = dot.parse(&written)?;
    mantissa.negative = negative;
    let exponent = if caps.get(2).is_some() {
        text_of(caps, 2).parse::<i32>().ok()?
    } else if caps.get(3).is_some() {
        superscript_int(text_of(caps, 3))?
    } else {
        let e = text_of(caps, 4);
        if mantissa.fraction.is_empty() && !e.starts_with(['+', '-']) {
            return None;
        }
        e.parse::<i32>().ok()?
    };
    let mut end = group(caps, 0)?.end;
    let mut unit = None;
    if let Some(u) = caps.get(5) {
        end = u.start;
        if let Some(resolved) = c.unit(&text[u.clone()], true) {
            if !char_after(text, u.end).is_some_and(|ch| ch.is_alphanumeric() || is_script(ch)) {
                unit = Some(resolved);
                end = u.end;
            }
        }
    }
    if !digit_bounded(text, &(mantissa_range.start..end)) {
        return None;
    }
    Some(Match::new(
        mantissa_range.start..end,
        Token::Scientific {
            mantissa,
            exponent,
            unit,
        },
    ))
}

pub(crate) fn chemical<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !letter_free(text, &whole) || char_after(text, whole.end).is_some_and(is_script) {
        return None;
    }
    // After the last subscript at most one element follows (`H₂O`), so a
    // glued word (`C₂II`) is not part of the formula.
    let formula = &text[whole.clone()];
    let tail = formula.rsplit(is_script).next().unwrap_or("");
    if tail.chars().filter(char::is_ascii_uppercase).count() > 1
        || formula.contains("II")
        || formula.contains("IV")
    {
        return None;
    }
    // `…Mo.` is the weekday abbreviation, not molybdenum.
    if char_after(text, whole.end) == Some('.') && c.abbreviation_key(&format!("{tail}.")).is_some()
    {
        return None;
    }
    let mut parts = Vec::new();
    let mut symbol = String::new();
    let mut charge_magnitude = None;
    for ch in formula.chars() {
        if ch.is_ascii_alphabetic() {
            if ch.is_ascii_uppercase() && !symbol.is_empty() {
                parts.push(ChemicalPart::Symbol(std::mem::take(&mut symbol)));
            }
            symbol.push(ch);
            continue;
        }
        if !symbol.is_empty() {
            parts.push(ChemicalPart::Symbol(std::mem::take(&mut symbol)));
        }
        if let Some(d) = subscript_digit(ch) {
            match parts.last_mut() {
                Some(ChemicalPart::Count(n)) => *n = *n * 10 + d,
                _ => parts.push(ChemicalPart::Count(d)),
            }
        } else if let Some(d) = superscript_digit(ch) {
            charge_magnitude = Some(charge_magnitude.unwrap_or(0) * 10 + d);
        } else {
            parts.push(ChemicalPart::Charge {
                magnitude: charge_magnitude.take(),
                negative: ch == '⁻',
            });
        }
    }
    if !symbol.is_empty() {
        parts.push(ChemicalPart::Symbol(symbol));
    }
    if !parts.iter().any(|p| !matches!(p, ChemicalPart::Symbol(_))) {
        return None;
    }
    // `I₂` would read "I zwei" and then "eins zwei" on a second pass (§9).
    let leading: String = formula
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if roman_value(&leading).is_some() {
        return None;
    }
    Some(Match::new(whole, Token::Chemical(parts)))
}

pub(crate) fn power<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let base = if let Some(n) = group(caps, 1) {
        let (numeral, range) = numeral_at(c.numerals(), text, &n)?;
        if !letter_free(text, &range) {
            return None;
        }
        PowerBase::Number(numeral)
    } else {
        let letter = text_of(caps, 2).chars().next()?;
        if !letter_free(text, &whole)
            || letter == 'I'
            // A hyphenated `L-a²` is not a variable `a` (Romanian `L-a` is an ordinal).
            || char_before(text, whole.start).is_some_and(|c| is_script(c) || c == '-')
        {
            return None;
        }
        // `m²`, `cm³` are unit symbols, not powers (§14).
        if c.units().resolve(&text[whole.clone()], true).is_some() {
            return None;
        }
        PowerBase::Variable(letter)
    };
    if char_after(text, whole.end).is_some_and(char::is_alphanumeric) {
        return None;
    }
    let exponent = match caps.get(3) {
        Some(_) => superscript_int(text_of(caps, 3))?,
        None => text_of(caps, 4).parse::<i32>().ok()?,
    };
    let start = match &base {
        PowerBase::Number(_) => signed(text, &group(caps, 1)?).1.start,
        PowerBase::Variable(_) => whole.start,
    };
    Some(Match::new(
        start..whole.end,
        Token::Power { base, exponent },
    ))
}

/// `UTC+2`, `GMT-5`: the zone name as written, the offset as an expression.
pub(crate) fn zone_offset<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !word_bounded(text, &whole) {
        return None;
    }
    let hours: u8 = text_of(caps, 3).parse().ok()?;
    let sign = if text_of(caps, 2) == "+" {
        Operator::Plus
    } else {
        Operator::Minus
    };
    Some(Match::new(
        whole,
        Token::Math(vec![
            MathItem::Word(text_of(caps, 1).to_string()),
            MathItem::Operator(sign),
            MathItem::Number(hours.into()),
        ]),
    ))
}

pub(crate) fn math<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let decimal = c.numerals().decimal;
    let mut items = Vec::new();
    let mut spans: Vec<Range<usize>> = Vec::new();
    let s = &text[whole.clone()];
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let ch = rest.chars().next()?;
        let at = whole.start + i;
        if ch.is_whitespace() {
            i += ch.len_utf8();
            continue;
        }
        let sign = ch == '-' || ch == '−';
        if ch.is_ascii_digit()
            || (sign && rest[ch.len_utf8()..].starts_with(|d: char| d.is_ascii_digit()))
        {
            let len = rest
                .find(|d: char| !(d.is_ascii_digit() || d == decimal || d == '-' || d == '−'))
                .unwrap_or(rest.len());
            let token = rest[..len].trim_end_matches(decimal);
            if sign && !matches!(items.last(), None | Some(MathItem::Operator(_))) {
                return None;
            }
            let mut n = c.numerals().parse(token.trim_start_matches(['-', '−']))?;
            n.negative = sign;
            let range = at..at + token.len();
            if !digit_bounded(text, &range) {
                return None;
            }
            spans.push(range);
            items.push(MathItem::Number(n));
            i += token.len();
            continue;
        }
        let op = match ch {
            '+' => Some(Operator::Plus),
            '−' | '-' => Some(Operator::Minus),
            '×' | '·' | '*' => Some(Operator::Times),
            '÷' => Some(Operator::DividedBy),
            '=' => Some(Operator::Equals),
            '≠' => Some(Operator::NotEquals),
            '<' => Some(Operator::Less),
            '>' => Some(Operator::Greater),
            '≤' => Some(Operator::LessOrEqual),
            '≥' => Some(Operator::GreaterOrEqual),
            '≈' => Some(Operator::Approximately),
            '±' => Some(Operator::PlusMinus),
            '√' => Some(Operator::SquareRoot),
            _ => None,
        };
        if let Some(op) = op {
            let prefix_ok = matches!(
                op,
                Operator::Less
                    | Operator::Greater
                    | Operator::LessOrEqual
                    | Operator::GreaterOrEqual
                    | Operator::PlusMinus
                    | Operator::SquareRoot
            );
            if matches!(items.last(), Some(MathItem::Operator(_)) | None) && !prefix_ok {
                return None;
            }
            spans.push(at..at + ch.len_utf8());
            items.push(MathItem::Operator(op));
            i += ch.len_utf8();
            continue;
        }
        let item = match ch {
            '∞' => MathItem::Infinity,
            'π' => MathItem::Pi,
            'I' => return None,
            ch if ch.is_alphabetic() => MathItem::Variable(ch),
            _ => return None,
        };
        let range = at..at + ch.len_utf8();
        if !word_bounded(text, &range) || char_after(text, range.end) == Some('.') {
            // A letter glued to a word (`Alter ≥65`) or before a period
            // (`z. B.`) is not an operand; drop it at either end.
            if items.is_empty() {
                i += ch.len_utf8();
                continue;
            }
            break;
        }
        spans.push(range);
        items.push(item);
        i += ch.len_utf8();
    }
    while matches!(items.last(), Some(MathItem::Operator(_))) {
        items.pop();
        spans.pop();
    }
    let numeric = items.iter().any(|it| matches!(it, MathItem::Number(_)));
    let single_infinity = items.len() == 1 && matches!(items[0], MathItem::Infinity);
    if (items.len() < 2 && !single_infinity) || (!numeric && !single_infinity) {
        return None;
    }
    let (start, mut end) = (spans.first()?.start, spans.last()?.end);
    if matches!(items.last(), Some(MathItem::Number(_))) {
        let rest = &text[end..];
        let after = rest.trim_start();
        let skipped = rest.len() - after.len();
        if skipped <= 1 && after.starts_with(['%', '‰']) {
            let sign = after.chars().next()?;
            let unit = c
                .units()
                .resolve_compound(c.numerals(), &sign.to_string(), true)?;
            items.push(MathItem::Unit(unit));
            end += skipped + sign.len_utf8();
        } else if let Some((unit, unit_end)) = trailing_unit(c, text, end) {
            items.push(MathItem::Unit(unit));
            end = unit_end;
        }
    }
    Some(Match::new(start..end, Token::Math(items)))
}

pub(crate) fn angle<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (degrees, degrees_range) = numeral_at(c.numerals(), text, &group(caps, 1)?)?;
    let minutes = caps
        .get(2)
        .map(|m| Numeral::int(text[m].parse().unwrap_or(0)));
    let seconds = caps
        .get(3)
        .map(|m| Numeral::int(text[m].parse().unwrap_or(0)));
    let mut end = group(caps, 0)?.end;
    let mut compass = None;
    if let Some(letter) = caps.get(4) {
        if char_after(text, letter.end).is_some_and(char::is_alphabetic) {
            end = letter.start;
            while text[..end].ends_with(char::is_whitespace) {
                end -= 1;
            }
        } else {
            compass = text[letter].chars().next();
        }
    }
    if minutes.is_none() && compass.is_none() {
        return None;
    }
    if !digit_bounded(text, &(degrees_range.start..end)) {
        return None;
    }
    Some(Match::new(
        degrees_range.start..end,
        Token::Angle {
            degrees,
            minutes,
            seconds,
            compass,
        },
    ))
}

pub(crate) fn blood_pressure<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let trigger =
        group(caps, 1).filter(|r| !char_before(text, r.start).is_some_and(char::is_alphanumeric));
    let mmhg = caps.get(4);
    if trigger.is_none() && mmhg.is_none() {
        return None;
    }
    let left = group(caps, 2)?;
    let right = group(caps, 3)?;
    let end = mmhg.as_ref().map_or(right.end, |m| m.end);
    let whole = left.start..end;
    if !digit_bounded(text, &whole) || char_after(text, end).is_some_and(char::is_alphanumeric) {
        return None;
    }
    let unit = mmhg.as_ref().and_then(|m| c.unit(&text[m.clone()], true));
    if mmhg.is_some() && unit.is_none() {
        return None;
    }
    let int = |r: Range<usize>| c.numerals().parse_int(&text[r]).map(Numeral::int);
    let token = Token::Ratio {
        left: int(left)?,
        right: int(right)?,
        unit,
        kind: RatioKind::BloodPressure,
    };
    Some(Match::new(whole, token))
}

pub(crate) fn range_rate<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) || char_before(text, whole.start).is_some_and(|ch| ch == '-') {
        return None;
    }
    let symbol = group(caps, 3)?;
    if char_after(text, symbol.end).is_some_and(char::is_alphanumeric) {
        return None;
    }
    let unit = c.unit(&format!("/{}", &text[symbol]), false)?;
    let int = |i: usize| c.numerals().parse_int(text_of(caps, i)).map(Numeral::int);
    let token = Token::Ratio {
        left: int(1)?,
        right: int(2)?,
        unit: Some(unit),
        kind: RatioKind::Range,
    };
    Some(Match::new(whole, token))
}

pub(crate) fn ratio<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let trigger = group(caps, 1).filter(|r| is_trigger_bounded(text, r));
    let left = group(caps, 2)?;
    let right = group(caps, 5)?;
    let whole = left.start..right.end;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let spaced = !text_of(caps, 3).is_empty() || !text_of(caps, 4).is_empty();
    let right_text = &text[right.clone()];
    let right_value = c.numerals().parse_int(right_text)?;
    let plausible_minute = right_text.len() == 2 && right_value <= 59;
    if trigger.is_none() && !spaced && (plausible_minute || right_text.len() < 2) {
        return None;
    }
    let token = Token::Ratio {
        left: Numeral::int(c.numerals().parse_int(&text[left])?),
        right: Numeral::int(right_value),
        unit: None,
        kind: RatioKind::Ratio,
    };
    Some(Match::new(whole, token))
}

pub(crate) fn dose<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let scheme_end = caps.get(1).map_or(whole.end, |u| {
        let mut end = u.start;
        while text[..end].ends_with(char::is_whitespace) {
            end -= 1;
        }
        end
    });
    let scheme = whole.start..scheme_end;
    if !digit_bounded(text, &scheme)
        || char_before(text, scheme.start).is_some_and(|ch| ch == '-' || ch.is_alphanumeric())
        || (char_after(text, scheme.end) == Some('-')
            && char_after(text, scheme.end + 1).is_some_and(|ch| ch.is_ascii_digit()))
    {
        return None;
    }
    let mut groups = Vec::new();
    for part in text[scheme.clone()].split('-') {
        if part == "½" {
            groups.push(Numeral {
                negative: false,
                integer: 0,
                fraction: "5".into(),
            });
        } else {
            let n = c.numerals().parse_int(part)?;
            if n > 200 {
                return None;
            }
            groups.push(Numeral::int(n));
        }
    }
    let mut end = scheme.end;
    let mut unit = None;
    if let Some(u) = caps.get(1) {
        if let Some(resolved) = c.unit(&text[u.clone()], true) {
            if !char_after(text, u.end).is_some_and(char::is_alphanumeric) {
                unit = Some(resolved);
                end = u.end;
            }
        }
    }
    Some(Match::new(scheme.start..end, Token::Dose { groups, unit }))
}

fn currency_code(tables: &Tables, written: &str) -> Option<&'static str> {
    tables
        .currencies
        .iter()
        .find(|c| c.written.contains(&written))
        .map(|c| c.code)
}

/// Groups: 1 = currency before the amount, 2 = amount, 3 = a scale word
/// (`Mio.`, `billion`), 4 = currency after, 5 = the word after a `/`
/// (`€/Monat`). A scale with no currency is [`Token::Scaled`]; a
/// one-letter scale (`$5k`) counts only glued to the amount and with a
/// currency.
pub(crate) fn money<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let prefix = caps.get(1);
    let suffix = caps.get(4);
    let amount_group = group(caps, 2)?;
    let (amount, amount_range) = numeral_at(c.numerals(), text, &amount_group)?;
    let scale = caps.get(3).and_then(|s| {
        let word = &text[s.clone()];
        let glued = s.start == amount_group.end;
        let bounded = !char_after(text, s.end).is_some_and(char::is_alphanumeric);
        let letter = word.chars().count() == 1;
        if !bounded || (letter && !glued) {
            return None;
        }
        c.scale(word).map(|scale| (scale, s.end, letter))
    });
    let (code, start, mut end) = match (&prefix, &suffix) {
        (Some(p), None) => (
            currency_code(c.tables(), &text[p.clone()])?,
            p.start,
            scale.map_or(amount_group.end, |(_, end, _)| end),
        ),
        (None, Some(s)) => {
            let written = &text[s.clone()];
            if written.chars().all(char::is_alphabetic)
                && char_after(text, s.end).is_some_and(char::is_alphanumeric)
            {
                return None;
            }
            (
                currency_code(c.tables(), written)?,
                amount_range.start,
                s.end,
            )
        }
        _ => {
            let (scale, end, letter) = scale?;
            if letter {
                return None;
            }
            let (unit, end) = match trailing_unit(c, text, end) {
                Some((unit, unit_end)) => (Some(unit), unit_end),
                None => (None, end),
            };
            let mut m = Match::new(
                amount_range.start..end,
                Token::Scaled {
                    amount,
                    scale,
                    unit,
                },
            );
            m.agreement = counted(c, text, end);
            return Some(m);
        }
    };
    let scale = scale.map(|(scale, _, _)| scale);
    let start = if prefix.is_some() {
        start
    } else {
        amount_range.start
    };
    let mut per = None;
    if let Some(p) = caps.get(5) {
        let word = &text[p.clone()];
        let part = c
            .units()
            .resolve_compound(c.numerals(), &format!("/{word}"), false)
            .and_then(|u| u.denominators.into_iter().next());
        match part {
            Some(part) if !char_after(text, p.end).is_some_and(char::is_alphanumeric) => {
                per = Some(part);
                end = p.end;
            }
            _ => {}
        }
    }
    if per.is_none() && char_after(text, end) == Some('/') {
        // `€/Monat` with an unknown word after the slash: keep it as written.
        if let Some((word, range)) = next_word(text, end + 1) {
            let plain_word = word.chars().count() >= 2
                && !word.chars().all(char::is_uppercase)
                && char_after(text, range.end) != Some('.');
            if range.start == end + 1 && plain_word {
                per = Some(UnitPart {
                    factor: None,
                    unit: UnitRef::Word(word.to_string()),
                });
                end = range.end;
            }
        }
    }
    Some(Match::new(
        start..end,
        Token::Money {
            amount,
            currency: code,
            per,
            scale,
        },
    ))
}

pub(crate) fn percent<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (value, range) = numeral_at(c.numerals(), text, &group(caps, 1)?)?;
    let permille = text_of(caps, 2) == "‰";
    Some(Match::new(
        range.start..group(caps, 0)?.end,
        Token::Percent { value, permille },
    ))
}

pub(crate) fn degrees<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (value, range) = numeral_at(c.numerals(), text, &group(caps, 1)?)?;
    let whole = group(caps, 0)?;
    let scale = match caps.get(2) {
        Some(s) if char_after(text, s.end).is_some_and(char::is_alphanumeric) => return None,
        Some(s) if &text[s.clone()] == "C" => Some("temperature-celsius"),
        Some(_) => Some("temperature-fahrenheit"),
        None => None,
    };
    Some(Match::new(
        range.start..whole.end,
        Token::Degrees { value, scale },
    ))
}

pub(crate) fn measure<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (value, range) = numeral_at(c.numerals(), text, &group(caps, 1)?)?;
    let glued = text_of(caps, 2).is_empty();
    let symbol = group(caps, 3)?;
    let mut end = group(caps, 0)?.end;
    let mut written = &text[symbol.start..end];
    let mut unit = c.unit(written, !written.contains('/'));
    if unit.is_none() {
        if let Some(stripped) = written.strip_suffix('.') {
            written = stripped;
            end -= 1;
            unit = c.unit(written, !written.contains('/'));
        }
    }
    let unit = unit?;
    if char_after(text, end).is_some_and(char::is_alphanumeric) {
        return None;
    }
    // `1/2 kg` is a slash fraction: a per-segment carries a number only
    // after a numerator unit (`ml/min/1,73 m²`).
    if unit.numerator.is_none() && written[1..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    if glued && unit.numerator.is_some() {
        // `500mg` is a symbol; `3,5m` and `1,5Liter` keep the input's missing space (§9).
        let numerator = written.split('/').next().unwrap_or("");
        let bare_symbol = matches!(
            unit.numerator,
            Some(UnitPart {
                unit: UnitRef::Cldr { .. } | UnitRef::Extra(_),
                ..
            })
        );
        if numerator.chars().count() < 2 || !bare_symbol {
            return None;
        }
    }
    Some(Match::new(
        range.start..end,
        Token::Measure {
            value,
            unit,
            hyphenated: false,
        },
    ))
}

pub(crate) fn hyphenated_measure<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let number = group(caps, 1)?;
    let symbol = group(caps, 2)?;
    if !digit_bounded(text, &number)
        || char_before(text, number.start).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let unit = c.unit(&text[symbol.clone()], true)?;
    let value = Numeral::int(c.numerals().parse_int(&text[number.clone()])?);
    Some(Match::new(
        number.start..symbol.end,
        Token::Measure {
            value,
            unit,
            hyphenated: true,
        },
    ))
}

/// Groups: 1 = count, 2 = an optional `/unit` after the times sign.
pub(crate) fn repetition<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (count, range) = numeral_at(c.numerals(), text, &group(caps, 1)?)?;
    let whole = group(caps, 0)?;
    let mut end = whole.end;
    let mut per = None;
    if let Some(p) = caps.get(2) {
        end = p.start;
        if let Some(unit) = c.unit(&text[p.clone()], false) {
            if !char_after(text, p.end).is_some_and(char::is_alphanumeric) {
                per = Some(unit);
                end = p.end;
            }
        }
    }
    if per.is_none() {
        match char_after(text, end) {
            None | Some('/') => {}
            Some(ch) if ch.is_whitespace() => {}
            Some(ch) if ch.is_ascii_digit() => {
                // `2x500` is a bare number; `0x1F` is not.
                let run = text[end..].trim_start_matches(|d: char| d.is_ascii_digit());
                if run.starts_with(char::is_alphabetic) {
                    return None;
                }
            }
            Some(_) => return None,
        }
    }
    if count.is_integer() && count.integer == 0 {
        return None;
    }
    Some(Match::new(
        range.start..end,
        Token::Repetition { count, per },
    ))
}

/// `A–B` of cardinals or years (ordinal ranges are per language). The
/// agreement is the noun after the range (`20–30 minute`), as for a
/// cardinal.
pub(crate) fn range<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let from = group(caps, 1)?;
    let to = group(caps, 2)?;
    if !digit_bounded(text, &whole)
        || char_before(text, whole.start).is_some_and(|ch| ch == '-' || ch == '–')
        || (char_after(text, whole.end) == Some('-')
            && char_after(text, whole.end + 1).is_some_and(|ch| ch.is_ascii_digit()))
    {
        return None;
    }
    let end = |r: &Range<usize>| -> Option<RangeEnd> {
        let value = c.numerals().parse_int(&text[r.clone()])?;
        Some(match u64::try_from(value) {
            Ok(year) if r.len() == 4 && c.is_year(year) => RangeEnd::Year(year),
            _ => RangeEnd::Cardinal(Numeral::int(value)),
        })
    };
    let agreement = counted(c, text, whole.end);
    let mut m = Match::new(
        whole,
        Token::Range {
            from: end(&from)?,
            to: end(&to)?,
        },
    );
    m.agreement = agreement;
    Some(m)
}

/// `2.3`, `3.10`, `1.2.7`: digit groups joined by periods. A shape the
/// language reads as a decimal (`2.3` in English) or a grouped cardinal
/// (`1.000` in German) is left to that class; dates outrank this one.
pub(crate) fn dotted<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let numerals = c.numerals();
    let mut groups = text[whole.clone()].split('.');
    let first = groups.next()?;
    let rest: Vec<&str> = groups.collect();
    if numerals.decimal == '.' && rest.len() == 1 {
        return None;
    }
    if numerals.groups.contains(&'.') && first.len() <= 3 && rest.iter().all(|g| g.len() == 3) {
        return None;
    }
    let groups = std::iter::once(first)
        .chain(rest)
        .map(str::to_string)
        .collect();
    Some(Match::new(whole, Token::Dotted(groups)))
}

pub(crate) fn year<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let digits = group(caps, 1)?;
    if !digit_bounded(text, &digits) || text[digits.clone()].starts_with('0') {
        return None;
    }
    if matches!(char_before(text, digits.start), Some('.' | ','))
        && char_before(text, digits.start - 1).is_some_and(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    let year: u64 = text[digits.clone()].parse().ok()?;
    if !c.is_year(year) {
        return None;
    }
    let decade = caps.get(2).is_some_and(|r| !r.is_empty());
    Some(Match::new(group(caps, 0)?, Token::Year { year, decade }))
}

pub(crate) fn decimal<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let (value, range) = numeral_at(c.numerals(), text, &whole)?;
    Some(Match::new(range, Token::Decimal(value)))
}

pub(crate) fn slash_fraction<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let before = char_before(text, whole.start);
    let after = char_after(text, whole.end);
    if before.is_some_and(|ch| ch.is_alphanumeric() || "€$£%/".contains(ch)) || after == Some('/')
    {
        return None;
    }
    let numerator: u32 = text_of(caps, 1).parse().ok()?;
    let denominator: u32 = text_of(caps, 2).parse().ok()?;
    // Denominators up to 99 derive their noun from the ordinal (§7.6);
    // larger ones only when the table names them (100, 1000).
    if numerator >= denominator
        || !(denominator <= 99
            || c.tables()
                .fractions
                .iter()
                .any(|(d, _, _)| *d == denominator))
    {
        return None;
    }
    Some(Match::new(
        whole,
        Token::Fraction {
            whole: None,
            numerator,
            denominator,
        },
    ))
}

pub(crate) fn mixed_fraction<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let number = group(caps, 1)?;
    if !digit_bounded(text, &number)
        || char_before(text, number.start).is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    let ch = text_of(caps, 2).chars().next()?;
    let (_, numerator, denominator) = *c
        .tables()
        .unicode_fractions
        .iter()
        .find(|(f, _, _)| *f == ch)?;
    let n = Numeral::int(c.numerals().parse_int(&text[number])?);
    Some(Match::new(
        whole,
        Token::Fraction {
            whole: Some(n),
            numerator,
            denominator,
        },
    ))
}

pub(crate) fn score<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !digit_bounded(text, &whole) {
        return None;
    }
    let left = text_of(caps, 1).parse().ok()?;
    let right = text_of(caps, 2).parse().ok()?;
    Some(Match::new(whole, Token::Score { left, right }))
}

/// CLDR's rules end at 10^18; beyond that the reading is digit by digit.
const SPELLABLE: u128 = 1_000_000_000_000_000_000;

/// §7.3/§7.5: what a number ending at `at` agrees with — the gender of the
/// noun it counts, and whether that noun follows it directly (so a
/// language may insert its linking word) or the linking word is written.
fn counted<C: Context>(c: &C, text: &str, at: usize) -> Agreement {
    let mut agreement = Agreement::default();
    if let Some((word, linked)) = c.counted_noun(text, at) {
        if text[at..].starts_with(char::is_whitespace) {
            agreement.gender = c.noun_gender(word);
            if c.noun_follows(word) && !linked {
                agreement.noun = NounPosition::After;
            }
        }
    }
    agreement
}

pub(crate) fn cardinal<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let (negative, range) = signed(text, &group(caps, 1)?);
    if !digit_bounded(text, &range) {
        return None;
    }
    let written = text[range.clone()].trim_start_matches(['-', '−']);
    let digits: String = written.chars().filter(char::is_ascii_digit).collect();
    let leading_zero = digits.len() > 1 && digits.starts_with('0');
    match c.numerals().parse_int(written) {
        Some(value) if !leading_zero && value < SPELLABLE => {
            let mut m = Match::new(
                range.clone(),
                Token::Cardinal(Numeral {
                    negative,
                    integer: value,
                    fraction: String::new(),
                }),
            );
            m.agreement = counted(c, text, range.end);
            Some(m)
        }
        _ => Some(Match::new(
            if negative {
                range.start + 1..range.end
            } else {
                range
            },
            Token::Digits(digits),
        )),
    }
}

pub(crate) fn abbreviation<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    if !letter_free(text, &whole) {
        return None;
    }
    let key = c.abbreviation_key(&text[whole.clone()])?;
    let agreement = c.abbreviation_context(key, text, &whole)?;
    let mut m = Match::new(whole, Token::Abbreviation(key.to_string()));
    m.agreement = agreement;
    Some(m)
}

pub(crate) fn unicode_fraction<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let ch = text[whole.clone()].chars().next()?;
    let (_, numerator, denominator) = *c
        .tables()
        .unicode_fractions
        .iter()
        .find(|(f, _, _)| *f == ch)?;
    Some(Match::new(
        whole,
        Token::Fraction {
            whole: None,
            numerator,
            denominator,
        },
    ))
}

pub(crate) fn roman<C: Context>(c: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let glued_script = char_before(text, whole.start).is_some_and(is_script)
        || char_after(text, whole.end).is_some_and(is_script);
    if !letter_free(text, &whole) || glued_script {
        return None;
    }
    let from = roman_value(text_of(caps, 1))?;
    let to = match caps.get(2) {
        Some(m) => Some(roman_value(&text[m])?),
        None => None,
    };
    // A lone `I` is a numeral only right after a grading word (`Billroth I`,
    // `Klasse-I-Empfehlung`), never on its own.
    if from == 1 && to.is_none() {
        let head = text[..whole.start].trim_end_matches([' ', '-', '\u{2011}']);
        let word = head
            .rsplit(|ch: char| !ch.is_alphabetic())
            .next()
            .unwrap_or("");
        if head.len() == whole.start || !c.tables().roman_context.contains(&word) {
            return None;
        }
    }
    Some(Match::new(whole, Token::Roman { from, to }))
}

pub(crate) fn paragraph<C: Context>(_: &C, text: &str, caps: &Caps<'_>) -> Option<Match> {
    let whole = group(caps, 0)?;
    let plural = text[whole.clone()].starts_with("§§");
    Some(Match::new(whole, Token::Paragraph { plural }))
}

// --------------------------------------------------------- spoken forms

/// `Kilometer`, `Millimol pro Liter`, `pro Minute`.
pub(crate) fn unit_words<C: Context>(c: &C, unit: &Unit, n: &Numeral) -> String {
    let units = c.units();
    let mut out = String::new();
    if let Some(numerator) = &unit.numerator {
        out.push_str(&units.name(&numerator.unit, n, Case::Nominative));
    }
    for part in &unit.denominators {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(units.per());
        out.push(' ');
        match &part.factor {
            Some(factor) => {
                out.push_str(&c.words(factor, None));
                out.push(' ');
                out.push_str(&units.name(&part.unit, factor, Case::Nominative));
            }
            None => out.push_str(&units.per_name(&part.unit)),
        }
    }
    out
}

/// `H zwei O`, `Ca zwei plus`: letters as written, counts and charges as words.
pub(crate) fn chemical_words<C: Context>(c: &C, parts: &[ChemicalPart]) -> String {
    let mut out = String::new();
    let mut last_was_symbol = false;
    for part in parts {
        match part {
            ChemicalPart::Symbol(s) => {
                if !out.is_empty() && !last_was_symbol {
                    out.push(' ');
                }
                out.push_str(s);
                last_was_symbol = true;
            }
            ChemicalPart::Count(n) => {
                out.push(' ');
                out.push_str(&c.words(&Numeral::from(*n), None));
                last_was_symbol = false;
            }
            ChemicalPart::Charge {
                magnitude,
                negative,
            } => {
                if let Some(m) = magnitude {
                    out.push(' ');
                    out.push_str(&c.words(&Numeral::from(*m), None));
                }
                out.push(' ');
                out.push_str(if *negative {
                    c.tables().minus
                } else {
                    c.tables().plus
                });
                last_was_symbol = false;
            }
        }
    }
    out
}

/// `2.3` → "zwei Punkt drei": each group as a cardinal, a leading-zero
/// group digit by digit (`1.01` → "eins Punkt null eins").
pub(crate) fn dotted_words<C: Context>(c: &C, language: Language, groups: &[String]) -> String {
    let group_words = |g: &String| match g.parse::<u128>() {
        Ok(value) if !(g.len() > 1 && g.starts_with('0')) && value < SPELLABLE => {
            c.words(&Numeral::int(value), None)
        }
        _ => spell::digits(language, g),
    };
    let mut out = String::new();
    for (i, g) in groups.iter().enumerate() {
        if i > 0 {
            out.push(' ');
            out.push_str(c.tables().point);
            out.push(' ');
        }
        out.push_str(&group_words(g));
    }
    out
}

/// `max@example.de` → "max at example Punkt de": letter runs as written,
/// digits one by one, symbols by the language's names.
pub(crate) fn electronic_words<C: Context>(c: &C, language: Language, written: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut Vec<String>| {
        if !run.is_empty() {
            out.push(std::mem::take(run));
        }
    };
    for ch in written.chars() {
        if ch.is_alphabetic() {
            run.push(ch);
        } else if let Some(d) = ch.to_digit(10) {
            flush(&mut run, &mut out);
            out.push(spell::digits(language, &d.to_string()));
        } else {
            flush(&mut run, &mut out);
            match c.tables().electronic.iter().find(|(s, _)| *s == ch) {
                Some((_, word)) => out.push((*word).to_string()),
                None => out.push(ch.to_string()),
            }
        }
    }
    flush(&mut run, &mut out);
    out.join(" ")
}

/// The exponent or charge magnitude as a signed numeral.
pub(crate) fn signed_int(n: i32) -> Numeral {
    Numeral {
        negative: n < 0,
        integer: n.unsigned_abs().into(),
        fraction: String::new(),
    }
}
