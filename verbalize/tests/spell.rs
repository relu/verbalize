//! `verbalize::spell` against the vendored CLDR data. Fidelity of the
//! interpreter itself is `tools/icu-diff`'s job; these pin the agreement →
//! ruleset mapping and the fallbacks.

use verbalize::spell;
use verbalize::token::{Agreement, Case, Declension, Gender};
use verbalize::Language::{self, De, En, Ro};

fn with(gender: Option<Gender>, case: Case, declension: Declension) -> Agreement {
    Agreement {
        gender,
        case,
        declension,
        ..Agreement::default()
    }
}

#[test]
fn cardinal() {
    let bare = Agreement::default();
    assert_eq!(
        spell::cardinal(De, 1_200_000, bare),
        "eine Million zweihunderttausend"
    );
    assert_eq!(spell::cardinal(De, 1, bare), "eins");
    assert_eq!(
        spell::cardinal(
            De,
            1,
            with(Some(Gender::Feminine), Case::Nominative, Declension::Strong)
        ),
        "eine"
    );
    assert_eq!(spell::cardinal(De, -21, bare), "minus einundzwanzig");
    assert_eq!(
        spell::cardinal(En, 1_234_567, bare),
        "one million two hundred thirty-four thousand five hundred sixty-seven"
    );
    assert_eq!(
        spell::cardinal(
            Ro,
            2,
            with(Some(Gender::Feminine), Case::Nominative, Declension::Strong)
        ),
        "două"
    );
    assert_eq!(
        spell::cardinal(
            Ro,
            22,
            with(Some(Gender::Feminine), Case::Nominative, Declension::Strong)
        ),
        "douăzeci și două"
    );
    assert_eq!(spell::cardinal(Ro, 2, bare), "doi");
}

#[test]
fn ordinal_endings_follow_the_declension_table() {
    use Case::*;
    use Declension::*;
    let m = Some(Gender::Masculine);
    let f = Some(Gender::Feminine);
    let n = Some(Gender::Neuter);
    // "am ersten November", "der erste", "erster November", "seit erstem".
    assert_eq!(spell::ordinal(De, 1, with(m, Dative, Weak)), "ersten");
    assert_eq!(spell::ordinal(De, 1, with(m, Nominative, Weak)), "erste");
    assert_eq!(
        spell::ordinal(De, 1, with(None, Nominative, Strong)),
        "erster"
    );
    assert_eq!(spell::ordinal(De, 1, with(m, Dative, Strong)), "erstem");
    assert_eq!(spell::ordinal(De, 3, with(f, Nominative, Strong)), "dritte");
    assert_eq!(
        spell::ordinal(De, 3, with(n, Nominative, Strong)),
        "drittes"
    );
    assert_eq!(spell::ordinal(De, 3, with(f, Dative, Strong)), "dritter");
    assert_eq!(
        spell::ordinal(De, 21, with(m, Genitive, Strong)),
        "einundzwanzigsten"
    );
    assert_eq!(
        spell::ordinal(En, 22, Agreement::default()),
        "twenty-second"
    );
}

#[test]
fn year() {
    assert_eq!(spell::year(En, 1990), "nineteen ninety");
    assert_eq!(spell::year(En, 2005), "two thousand five");
    assert_eq!(spell::year(De, 1990), "neunzehnhundertneunzig");
    assert_eq!(spell::year(De, 2024), "zweitausendvierundzwanzig");
}

#[test]
fn digits() {
    assert_eq!(
        spell::digits(De, "01234/56789"),
        "null eins zwei drei vier / fünf sechs sieben acht neun"
    );
    assert_eq!(spell::digits(En, "+49 30"), "+ four nine three zero");
    assert_eq!(spell::digits(De, ""), "");
}

#[test]
fn missing_ruleset_falls_back_to_the_default_ruleset() {
    // CLDR ro has no %spellout-ordinal-*; the default ruleset reads it.
    assert_eq!(spell::ordinal(Ro, 2, Agreement::default()), "doi");
    assert_eq!(spell::ruleset(Ro, "%spellout-ordinal-masculine", 2), None);
}

#[test]
fn ruleset_by_name() {
    assert_eq!(
        spell::ruleset(De, "%spellout-cardinal-n", 1).as_deref(),
        Some("einen")
    );
    assert_eq!(spell::ruleset(De, "%%ste", 3).as_deref(), Some("dritte"));
    assert_eq!(
        spell::ruleset(En, "%spellout-ordinal", 10u64.pow(18).into()).as_deref(),
        Some("1,000,000,000,000,000,000st")
    );
    assert_eq!(
        spell::ruleset(De, "%spellout-numbering", 10u64.pow(18).into()).as_deref(),
        Some("1.000.000.000.000.000.000")
    );
}

#[test]
fn language_from_bcp47() {
    assert_eq!("de".parse(), Ok(De));
    assert_eq!("de-AT".parse(), Ok(De));
    assert_eq!("EN-us".parse(), Ok(En));
    assert_eq!("ro-Latn-RO".parse(), Ok(Ro));
    assert!("fr".parse::<Language>().is_err());
    assert!(
        "de_DE".parse::<Language>().is_err(),
        "underscores are not BCP-47"
    );
    assert!("".parse::<Language>().is_err());
}
