//! Number spelling without classification, for callers that already know
//! what they have (design §5). Stateless: the compiled rulesets of each
//! language live in a `LazyLock`.

use std::sync::LazyLock;

use crate::rbnf::{Rbnf, Value};
use crate::token::{Agreement, Case, Declension, Gender};
use crate::Language;

/// Cardinal number as words: `cardinal(De, 1_200_000, Agreement::default())`
/// → "eine Million zweihunderttausend". A known gender selects the
/// inflected "ein/eine" forms; the default is the standalone "eins".
pub fn cardinal(language: Language, n: i128, agreement: Agreement) -> String {
    spell(
        language,
        cardinal_ruleset(language, agreement),
        Value::int(n),
    )
}

/// Ordinal number as words with the ending the context requires:
/// `ordinal(De, 1, dative masculine after a determiner)` → "ersten".
pub fn ordinal(language: Language, n: u64, agreement: Agreement) -> String {
    spell(
        language,
        ordinal_ruleset(language, agreement),
        Value::int(i128::from(n)),
    )
}

/// Year as spoken: `year(En, 1990)` → "nineteen ninety",
/// `year(De, 1990)` → "neunzehnhundertneunzig".
pub fn year(language: Language, y: u64) -> String {
    spell(
        language,
        "%spellout-numbering-year",
        Value::int(i128::from(y)),
    )
}

/// Digit by digit, for telephone numbers and codes: `digits(De, "01/23")`
/// → "null eins / zwei drei". Every non-digit character is kept as its
/// own word; whitespace only separates.
pub fn digits(language: Language, s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_whitespace() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        match c.to_digit(10) {
            Some(d) => out.push_str(&spell(
                language,
                "%spellout-numbering",
                Value::int(d.into()),
            )),
            None => out.push(c),
        }
    }
    out
}

/// Spells `n` with a named CLDR ruleset (`"%spellout-cardinal-feminine"`,
/// private `%%` names included), bypassing agreement. `None` when the
/// language has no such ruleset. For tooling and callers that know the
/// exact rules they want.
pub fn ruleset(language: Language, ruleset: &str, n: i128) -> Option<String> {
    rbnf(language).format(ruleset, Value::int(n))
}

pub(crate) fn spell(language: Language, ruleset: &str, n: Value<'_>) -> String {
    let rbnf = rbnf(language);
    rbnf.format(ruleset, n)
        .or_else(|| rbnf.format(rbnf.default_ruleset(), n))
        .unwrap_or_else(|| plain(n))
}

/// Last resort when the rules cannot format a value: the digits.
fn plain(n: Value<'_>) -> String {
    let sign = if n.negative { "-" } else { "" };
    if n.fraction.is_empty() {
        format!("{sign}{}", n.integer)
    } else {
        format!("{sign}{}.{}", n.integer, n.fraction)
    }
}

pub(crate) fn rbnf(language: Language) -> &'static Rbnf {
    static DE: LazyLock<Rbnf> = LazyLock::new(|| compile(Language::De));
    static EN: LazyLock<Rbnf> = LazyLock::new(|| compile(Language::En));
    static RO: LazyLock<Rbnf> = LazyLock::new(|| compile(Language::Ro));
    match language {
        Language::De => &DE,
        Language::En => &EN,
        Language::Ro => &RO,
    }
}

fn compile(language: Language) -> Rbnf {
    // `cargo xtask gen` parses the vendored rules with the same parser
    // before they are checked in, so this cannot fail on shipped data.
    Rbnf::new(language.data())
        .unwrap_or_else(|e| panic!("vendored RBNF data for {}: {e}", language.code()))
}

/// Which cardinal ruleset the agreement selects. Gender-inflected "ein"
/// forms only when the gender is known (design §7.3, §7.5).
pub(crate) fn cardinal_ruleset(language: Language, agreement: Agreement) -> &'static str {
    match (language, agreement.gender) {
        (Language::En, _) | (_, None) => "%spellout-numbering",
        (Language::De | Language::Ro, Some(Gender::Masculine)) => "%spellout-cardinal-masculine",
        (Language::De | Language::Ro, Some(Gender::Feminine)) => "%spellout-cardinal-feminine",
        (Language::De | Language::Ro, Some(Gender::Neuter)) => "%spellout-cardinal-neuter",
    }
}

/// Which ordinal ruleset the agreement selects. German has one ruleset per
/// adjective ending (design §7.3): the standard strong/weak declension
/// table, with masculine as the default gender (months are masculine).
pub(crate) fn ordinal_ruleset(language: Language, agreement: Agreement) -> &'static str {
    use Case::*;
    use Gender::*;
    match language {
        Language::En => "%spellout-ordinal",
        Language::Ro => match agreement.gender {
            Some(Feminine) => "%spellout-ordinal-feminine",
            _ => "%spellout-ordinal-masculine",
        },
        Language::De => {
            let gender = agreement.gender.unwrap_or(Masculine);
            match (agreement.declension, agreement.case, gender) {
                (Declension::Weak, Nominative, _) => "%spellout-ordinal",
                (Declension::Weak, Accusative, Feminine | Neuter) => "%spellout-ordinal",
                (Declension::Weak, _, _) => "%spellout-ordinal-n",
                (Declension::Strong, Nominative, Masculine) => "%spellout-ordinal-r",
                (Declension::Strong, Nominative | Accusative, Feminine) => "%spellout-ordinal",
                (Declension::Strong, Nominative | Accusative, Neuter) => "%spellout-ordinal-s",
                (Declension::Strong, Accusative, Masculine) => "%spellout-ordinal-n",
                (Declension::Strong, Dative, Masculine | Neuter) => "%spellout-ordinal-m",
                (Declension::Strong, Dative | Genitive, Feminine) => "%spellout-ordinal-r",
                (Declension::Strong, Genitive, Masculine | Neuter) => "%spellout-ordinal-n",
            }
        }
    }
}
