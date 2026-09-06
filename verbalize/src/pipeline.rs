//! classify → verbalize → render (design §4, §9).
//!
//! A language is a priority-ordered list of [`Recognizer`]s. Each runs its
//! regex over the whole input; a candidate is kept only if it does not
//! overlap a span a higher-priority recogniser already claimed, so lower
//! priorities never see text inside a claimed span. Per-recogniser
//! `Regex` plus this merge step is the §14 fallback: context words and
//! digit-boundary checks need more than a `RegexSet` membership test.

use std::ops::Range;

use regex::{CaptureLocations, Regex, RegexSet};

use crate::lang::Lang;
use crate::token::{Agreement, Span, Token};
use crate::Options;

/// One semiotic-class recogniser: a regex and the function that turns a
/// match into a token (or rejects it). `C` is the language's compiled
/// context (tables, lexicon).
pub(crate) struct Recognizer<C> {
    pub regex: Regex,
    pub parse: fn(&C, &str, &Caps<'_>) -> Option<Match>,
}

/// The capture groups of one candidate match, without allocating per match.
pub(crate) struct Caps<'a> {
    locations: &'a CaptureLocations,
    text: &'a str,
}

impl<'a> Caps<'a> {
    /// Byte range of group `i`, if it participated.
    pub(crate) fn get(&self, i: usize) -> Option<Range<usize>> {
        self.locations.get(i).map(|(start, end)| start..end)
    }

    /// Text of group `i`, empty if it did not participate.
    pub(crate) fn text(&self, i: usize) -> &'a str {
        self.get(i).map_or("", |r| &self.text[r])
    }

    /// Start of the first participating group among `i..`.
    pub(crate) fn first_start(&self, from: usize) -> Option<usize> {
        (from..self.locations.len())
            .find_map(|i| self.get(i))
            .map(|r| r.start)
    }
}

/// The recognisers of a language with their `RegexSet` prefilter, so a
/// text is scanned once to know which recognisers can match at all.
pub(crate) struct Recognizers<C> {
    list: Vec<Recognizer<C>>,
    set: RegexSet,
}

impl<C> Recognizers<C> {
    pub(crate) fn new(list: Vec<Recognizer<C>>) -> Self {
        let set = RegexSet::new(list.iter().map(|r| r.regex.as_str()))
            .expect("patterns compiled individually");
        Recognizers { list, set }
    }
}

/// A classified span before verbalization.
#[derive(Clone, Debug)]
pub(crate) struct Match {
    pub range: Range<usize>,
    pub token: Token,
    pub agreement: Agreement,
}

impl Match {
    pub(crate) fn new(range: Range<usize>, token: Token) -> Self {
        Match {
            range,
            token,
            agreement: Agreement::default(),
        }
    }
}

/// Runs the recognisers in priority order; returns the claimed matches
/// sorted by position.
pub(crate) fn classify<C>(recognizers: &Recognizers<C>, ctx: &C, text: &str) -> Vec<Match> {
    let mut claimed = Claimed::default();
    let mut matches = Vec::new();
    for index in recognizers.set.matches(text) {
        let recognizer = &recognizers.list[index];
        let mut locations = recognizer.regex.capture_locations();
        let mut at = 0;
        while let Some(found) = recognizer.regex.captures_read_at(&mut locations, text, at) {
            let caps = Caps {
                locations: &locations,
                text,
            };
            if let Some(m) = (recognizer.parse)(ctx, text, &caps) {
                debug_assert!(m.range.start < m.range.end, "empty span from {:?}", m.token);
                if claimed.claim(&m.range) {
                    matches.push(m);
                }
            }
            at = if found.end() > found.start() {
                found.end()
            } else {
                found.end() + 1
            };
            if at > text.len() {
                break;
            }
        }
    }
    matches.sort_by_key(|m| m.range.start);
    matches
}

/// Claimed byte ranges, kept sorted and disjoint.
#[derive(Default)]
struct Claimed(Vec<Range<usize>>);

impl Claimed {
    fn claim(&mut self, range: &Range<usize>) -> bool {
        let at = self.0.partition_point(|r| r.end <= range.start);
        if self.0.get(at).is_some_and(|next| next.start < range.end) {
            return false;
        }
        self.0.insert(at, range.clone());
        true
    }
}

pub(crate) fn annotate(lang: &dyn Lang, options: &Options, text: &str) -> Vec<Span> {
    let mut matches = lang.classify(text);
    apply_lexicon(&options.lexicon, text, &mut matches);
    matches
        .into_iter()
        .map(|m| {
            let spoken = match &m.token {
                Token::Lexicon(key) => {
                    let entry = options.lexicon.iter().find(|(k, _)| k == key);
                    crate::lang::Verbalized::plain(
                        entry.map(|(_, v)| v.clone()).unwrap_or_default(),
                    )
                }
                token => lang.verbalize(token, m.agreement),
            };
            Span {
                range: m.range,
                token: m.token,
                spoken: spoken.spoken,
                fallback: spoken.fallback,
            }
        })
        .collect()
}

/// `Options::lexicon`: literal, case-sensitive, word-bounded entries over
/// the text the classes left unclaimed. An entry may take over bare
/// digit readings glued inside it (`SpO2`), never any other class. Longer
/// entries win over shorter.
fn apply_lexicon(lexicon: &[(String, String)], text: &str, matches: &mut Vec<Match>) {
    if lexicon.is_empty() {
        return;
    }
    let mut entries: Vec<&str> = lexicon
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| !k.is_empty())
        .collect();
    entries.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for key in entries {
        for (at, _) in text.match_indices(key) {
            let range = at..at + key.len();
            if !word_bounded(text, &range) {
                continue;
            }
            let overlapping = matches
                .iter()
                .filter(|m| m.range.start < range.end && range.start < m.range.end);
            let only_bare_digits = overlapping.clone().all(|m| {
                matches!(m.token, Token::Cardinal(_) | Token::Digits(_))
                    && range.start <= m.range.start
                    && m.range.end <= range.end
            });
            if !only_bare_digits {
                continue;
            }
            matches.retain(|m| !(m.range.start < range.end && range.start < m.range.end));
            matches.push(Match::new(range, Token::Lexicon(key.to_string())));
        }
    }
    matches.sort_by_key(|m| m.range.start);
}

/// Neither neighbour is a letter or digit.
pub(crate) fn word_bounded(text: &str, range: &Range<usize>) -> bool {
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

/// Splices the spoken forms over their byte ranges; everything outside a
/// span is byte-identical to the input (design §9). An integer reading
/// after an uppercase letter (`B1`, `SpO2`) gets a separating space so the
/// letter is spelled on its own, and one sandwiched between letters of any
/// case (`H2O`, `HbA1c`) on both sides; a suffix on a word that starts
/// with the digits stays attached (`630c`, `1990er`, `1F`).
pub(crate) fn render(text: &str, spans: &[Span]) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    let mut at = 0;
    for span in spans {
        out.push_str(&text[at..span.range.start]);
        let glue = matches!(span.token, Token::Cardinal(_) | Token::Digits(_));
        let before = text[..span.range.start].chars().next_back();
        let after = text[span.range.end..].chars().next();
        let sandwiched =
            before.is_some_and(char::is_alphabetic) && after.is_some_and(char::is_alphabetic);
        if glue && (sandwiched || before.is_some_and(char::is_uppercase)) {
            out.push(' ');
        }
        out.push_str(&span.spoken);
        if glue && sandwiched {
            out.push(' ');
        }
        at = span.range.end;
    }
    out.push_str(&text[at..]);
    out
}
