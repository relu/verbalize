//! The `=#,##0=` family: ICU falls back to a `DecimalFormat` pattern for
//! values a ruleset does not spell (10¹⁸ and above, fractional ordinals).
//! Only the pattern subset CLDR spellout rules use is accepted: optional
//! grouping, `0`/`#` digit placeholders, an optional fraction part.

use std::cmp::Ordering;

use super::{Symbols, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DecimalPattern {
    /// Digits per group, from the last `,` to the end of the integer part.
    grouping: Option<usize>,
    min_fraction: usize,
    max_fraction: usize,
}

impl DecimalPattern {
    pub(super) fn parse(pattern: &str) -> Result<Self, &'static str> {
        let (integer, fraction) = match pattern.split_once('.') {
            Some((i, f)) => (i, f),
            None => (pattern, ""),
        };
        if integer.is_empty() || !integer.bytes().all(|b| matches!(b, b'#' | b'0' | b',')) {
            return Err("unsupported decimal-format pattern");
        }
        if integer.matches(',').count() > 1 {
            return Err("secondary grouping is not supported");
        }
        let grouping = match integer.rfind(',') {
            Some(at) => Some(integer.len() - at - 1).filter(|&n| n > 0),
            None => None,
        };
        if integer.contains(',') && grouping.is_none() {
            return Err("unsupported decimal-format pattern");
        }
        let min_fraction = fraction.bytes().take_while(|&b| b == b'0').count();
        if !fraction[min_fraction..].bytes().all(|b| b == b'#') {
            return Err("unsupported decimal-format pattern");
        }
        Ok(DecimalPattern {
            grouping,
            min_fraction,
            max_fraction: fraction.len(),
        })
    }

    pub(super) fn format(&self, v: Value<'_>, symbols: &Symbols, out: &mut String) {
        let (integer, fraction) = round_half_even(v.integer, v.fraction, self.max_fraction);
        if v.negative {
            out.push_str(&symbols.minus);
        }
        let digits = integer.to_string();
        match self.grouping {
            Some(size) => {
                let head = digits.len() % size;
                let (head, tail) = digits.split_at(head);
                out.push_str(head);
                for (i, chunk) in tail.as_bytes().chunks(size).enumerate() {
                    if !head.is_empty() || i > 0 {
                        out.push_str(&symbols.group);
                    }
                    out.push_str(std::str::from_utf8(chunk).expect("ascii digits"));
                }
            }
            None => out.push_str(&digits),
        }
        let mut keep = fraction.len();
        while keep > self.min_fraction && fraction[keep - 1] == 0 {
            keep -= 1;
        }
        if keep > 0 || self.min_fraction > 0 {
            out.push_str(&symbols.decimal);
            out.extend(fraction[..keep].iter().map(|d| (b'0' + d) as char));
            out.extend(std::iter::repeat_n(
                '0',
                self.min_fraction.saturating_sub(keep),
            ));
        }
    }
}

/// `DecimalFormat`'s default rounding, on digit strings: keeps `max`
/// fraction digits, rounding ties to the even neighbour.
fn round_half_even(integer: u128, fraction: &str, max: usize) -> (u128, Vec<u8>) {
    let digits: Vec<u8> = fraction.bytes().map(|b| b - b'0').collect();
    if digits.len() <= max {
        return (integer, digits);
    }
    let (keep, rest) = digits.split_at(max);
    let mut keep = keep.to_vec();
    let last = keep.last().copied().unwrap_or((integer % 10) as u8);
    let up = match rest[0].cmp(&5) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => rest[1..].iter().any(|&d| d != 0) || last % 2 == 1,
    };
    if !up {
        return (integer, keep);
    }
    for d in keep.iter_mut().rev() {
        if *d == 9 {
            *d = 0;
        } else {
            *d += 1;
            return (integer, keep);
        }
    }
    (integer.saturating_add(1), keep)
}
