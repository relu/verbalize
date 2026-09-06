//! Contracts of the parser and interpreter on hand-written rulesets, one
//! per descriptor and substitution kind. Fidelity to ICU on the real CLDR
//! data is established by `tools/icu-diff`, not here.

use super::{ParseError, Rbnf, Symbols, Value};

fn compile(decimal: &str, rulesets: &[(&str, &[(&str, &str)])]) -> Result<Rbnf, ParseError> {
    let group = if decimal == "." { "," } else { "." };
    Rbnf::compile(
        "en",
        Symbols::new(decimal, group, "-"),
        rulesets.iter().copied(),
    )
}

fn rbnf(rulesets: &[(&str, &[(&str, &str)])]) -> Rbnf {
    compile(".", rulesets).unwrap_or_else(|e| panic!("{e}"))
}

fn int(rbnf: &Rbnf, ruleset: &str, n: i128) -> String {
    rbnf.format(ruleset, Value::int(n))
        .unwrap_or_else(|| panic!("{ruleset} cannot format {n}"))
}

fn dec(rbnf: &Rbnf, ruleset: &str, integer: u128, fraction: &str) -> String {
    let v = Value::decimal(false, integer, fraction);
    rbnf.format(ruleset, v)
        .unwrap_or_else(|| panic!("{ruleset} cannot format {integer}.{fraction}"))
}

const DIGITS: &[(&str, &str)] = &[
    ("0", "zero;"),
    ("1", "one;"),
    ("2", "two;"),
    ("3", "three;"),
    ("4", "four;"),
    ("5", "five;"),
    ("6", "six;"),
    ("7", "seven;"),
    ("8", "eight;"),
    ("9", "nine;"),
];

#[test]
fn brackets_split_rules_and_rollback_avoids_zero() {
    let r = rbnf(&[(
        "%x",
        &[
            ("0", "=#,##0=;"),
            ("20", "twenty[->>];"),
            ("100", "<< hundred[ >>];"),
            ("1000", "<< thousand[ >>];"),
        ],
    )]);
    assert_eq!(int(&r, "%x", 20), "twenty");
    assert_eq!(int(&r, "%x", 21), "twenty-1");
    assert_eq!(int(&r, "%x", 100), "1 hundred");
    assert_eq!(int(&r, "%x", 105), "1 hundred 5");
    assert_eq!(int(&r, "%x", 200), "2 hundred");
    assert_eq!(int(&r, "%x", 321), "3 hundred twenty-1");
    assert_eq!(int(&r, "%x", 2000), "2 thousand");
    assert_eq!(int(&r, "%x", 2100), "2 thousand 1 hundred");
    assert_eq!(int(&r, "%x", 1_000_000), "1 thousand thousand");
}

#[test]
fn bracket_alternative_text() {
    let r = rbnf(&[("%x", &[("0", "=#,##0=;"), ("20", "<<ty[->>|.];")])]);
    assert_eq!(int(&r, "%x", 20), "2ty.");
    assert_eq!(int(&r, "%x", 21), "2ty-1");
    assert_eq!(int(&r, "%x", 30), "3ty.");
}

#[test]
fn arrows_are_the_ascii_tokens() {
    let ascii = rbnf(&[("%x", &[("0", "=#,##0=;"), ("100", "<< hundred[ >>];")])]);
    let arrows = rbnf(&[("%x", &[("0", "=#,##0=;"), ("100", "←← hundred[ →→];")])]);
    assert_eq!(int(&ascii, "%x", 305), int(&arrows, "%x", 305));
}

#[test]
fn radix_descriptor_sets_the_divisor() {
    let r = rbnf(&[("%x", &[("0", "=#,##0=;"), ("12/12", "<< dozen[ >>];")])]);
    assert_eq!(int(&r, "%x", 24), "2 dozen");
    assert_eq!(int(&r, "%x", 25), "2 dozen 1");
    assert_eq!(int(&r, "%x", 144), "1 dozen dozen");
}

#[test]
fn exponent_adjustment_descriptor() {
    // 1100/100: divisor 100 spells 1990 as nineteen hundred ninety.
    let year = rbnf(&[
        ("%%d", &[("0", "=#,##0=;")]),
        (
            "%y",
            &[
                ("0", "=%%d=;"),
                ("1100/100", "<< hundred[ >>];"),
                ("2000", "=%%d=;"),
            ],
        ),
    ]);
    assert_eq!(int(&year, "%y", 1990), "19 hundred 90");
    assert_eq!(int(&year, "%y", 1900), "19 hundred");
    assert_eq!(int(&year, "%y", 2000), "2,000");
    // 100>: the exponent of 100 is 2, one > makes it 1, so the divisor is 10.
    let adjusted = rbnf(&[("%x", &[("0", "=#,##0=;"), ("100>", "<<|>>;")])]);
    assert_eq!(int(&adjusted, "%x", 123), "12|3");
    // 100>>: the exponent is 0 and the divisor 1 (<< must not recurse).
    let twice = rbnf(&[
        ("%%d", &[("0", "=#,##0=;")]),
        ("%x", &[("0", "=#,##0=;"), ("100>>", "<%%d<|>>;")]),
    ]);
    assert_eq!(int(&twice, "%x", 123), "123|0");
}

#[test]
fn three_arrows_use_the_preceding_rule() {
    let r = rbnf(&[(
        "%x",
        &[("0", "=#,##0=;"), ("9", "uno;"), ("10", "<<0->>>;")],
    )]);
    assert_eq!(int(&r, "%x", 21), "20-uno");
    assert_eq!(int(&r, "%x", 25), "20-uno");
}

#[test]
fn negative_rule_or_sign_kept_for_the_delegate() {
    let r = rbnf(&[
        ("%neg", &[("-x", "minus >>;"), ("0", "=#,##0.#=;")]),
        ("%delegate", &[("0", "=%neg=;")]),
        ("%decimal", &[("0", "=#,##0=;")]),
        ("%parts", &[("0", "=#,##0=;"), ("100", "<< hundred[ >>];")]),
    ]);
    assert_eq!(int(&r, "%neg", -5), "minus 5");
    assert_eq!(int(&r, "%neg", 5), "5");
    assert_eq!(
        r.format("%neg", Value::decimal(true, 0, "5")).unwrap(),
        "minus 0.5"
    );
    // No -x rule: the magnitude selects the rule, the signed value is
    // formatted, exactly as ICU (Java truncating division) does.
    assert_eq!(int(&r, "%delegate", -5), "minus 5");
    assert_eq!(int(&r, "%decimal", -5), "-5");
    assert_eq!(int(&r, "%parts", -250), "-2 hundred -50");
    assert_eq!(int(&r, "%parts", -200), "-2 hundred");
}

#[test]
fn fraction_rules_spell_digits_one_at_a_time() {
    let mut rules = vec![("x.x", "<< point >>;")];
    rules.extend_from_slice(DIGITS);
    let r = rbnf(&[("%x", &rules)]);
    assert_eq!(dec(&r, "%x", 3, "14"), "three point one four");
    assert_eq!(dec(&r, "%x", 3, "10"), "three point one zero");
    assert_eq!(dec(&r, "%x", 0, "5"), "zero point five");
    assert_eq!(int(&r, "%x", 3), "three");

    let mut rules = vec![("x.x", "<< point >>;"), ("0.x", "point >>;")];
    rules.extend_from_slice(DIGITS);
    let proper = rbnf(&[("%x", &rules)]);
    assert_eq!(dec(&proper, "%x", 0, "5"), "point five");
    assert_eq!(dec(&proper, "%x", 1, "5"), "one point five");

    let mut rules = vec![("x.x", "<< point >>>;")];
    rules.extend_from_slice(DIGITS);
    let joined = rbnf(&[("%x", &rules)]);
    assert_eq!(dec(&joined, "%x", 3, "14"), "three point onefour");
}

#[test]
fn fraction_rule_brackets_split_into_proper_rule() {
    let mut rules = vec![("x.x", "[<< ]point >>;")];
    rules.extend_from_slice(DIGITS);
    let r = rbnf(&[("%x", &rules)]);
    assert_eq!(dec(&r, "%x", 2, "5"), "two point five");
    assert_eq!(dec(&r, "%x", 0, "5"), "point five");
}

#[test]
fn master_rule_takes_fractions_and_rounds_half_even() {
    let mut rules = vec![("x.0", "=#,##0.#=;")];
    rules.extend_from_slice(DIGITS);
    let r = rbnf(&[("%x", &rules)]);
    assert_eq!(int(&r, "%x", 1), "one");
    assert_eq!(dec(&r, "%x", 1, "25"), "1.2");
    assert_eq!(dec(&r, "%x", 1, "35"), "1.4");
    assert_eq!(dec(&r, "%x", 1, "251"), "1.3");
    assert_eq!(dec(&r, "%x", 9, "96"), "10");
    assert_eq!(dec(&r, "%x", 1, "0"), "1");
}

#[test]
fn fraction_without_fraction_rules_rounds_half_up() {
    let r = rbnf(&[("%x", DIGITS)]);
    assert_eq!(dec(&r, "%x", 2, "4"), "two");
    assert_eq!(dec(&r, "%x", 2, "5"), "three");
}

#[test]
fn locale_decimal_point_selects_among_fraction_rules() {
    let rules: &[(&str, &[(&str, &str)])] = &[(
        "%x",
        &[("x.x", "dot;"), ("x,x", "comma;"), ("0", "=#,##0=;")],
    )];
    let point = compile(".", rules).unwrap();
    let comma = compile(",", rules).unwrap();
    assert_eq!(dec(&point, "%x", 1, "5"), "dot");
    assert_eq!(dec(&comma, "%x", 1, "5"), "comma");
}

#[test]
fn same_value_and_named_rulesets() {
    let r = rbnf(&[
        ("%%priv", &[("0", "P=#,##0=;")]),
        (
            "%pub",
            &[("0", "=%%priv=;"), ("100", "<%%priv< H >%%priv>;")],
        ),
    ]);
    assert_eq!(int(&r, "%pub", 7), "P7");
    assert_eq!(int(&r, "%pub", 207), "P2 H P7");
    assert_eq!(int(&r, "%%priv", 7), "P7");
    assert_eq!(r.default_ruleset(), "%pub");
}

#[test]
fn decimal_format_patterns() {
    let r = compile(
        ",",
        &[
            ("%grouped", &[("0", "=#,##0=;")]),
            ("%plain", &[("0", "=0=;")]),
            ("%cents", &[("0", "=#,##0.00=;")]),
            ("%tenths", &[("0", "=#,##0.#=;")]),
        ],
    )
    .unwrap();
    assert_eq!(int(&r, "%grouped", 0), "0");
    assert_eq!(int(&r, "%grouped", 999), "999");
    assert_eq!(int(&r, "%grouped", 1000), "1.000");
    assert_eq!(int(&r, "%grouped", 1_234_567), "1.234.567");
    assert_eq!(
        int(&r, "%grouped", 1_000_000_000_000_000_000_000),
        "1.000.000.000.000.000.000.000"
    );
    assert_eq!(dec(&r, "%grouped", 2, "5"), "2", "half-even to zero digits");
    assert_eq!(dec(&r, "%grouped", 3, "5"), "4");
    assert_eq!(int(&r, "%plain", 12345), "12345");
    assert_eq!(int(&r, "%cents", 25), "25,00");
    assert_eq!(dec(&r, "%cents", 25, "5"), "25,50");
    assert_eq!(dec(&r, "%cents", 25, "999"), "26,00");
    assert_eq!(int(&r, "%tenths", 35), "35");
    assert_eq!(dec(&r, "%tenths", 35, "5"), "35,5");
    assert_eq!(
        dec(&r, "%tenths", 35, "05"),
        "35",
        "half-even: 35.05 -> 35.0"
    );
    assert_eq!(dec(&r, "%tenths", 35, "15"), "35,2");
}

#[test]
fn plural_forms_by_category_of_the_quotient() {
    let r = rbnf(&[
        (
            "%o",
            &[("0", "=#,##0=$(ordinal,one{st}two{nd}few{rd}other{th})$;")],
        ),
        (
            "%c",
            &[
                ("0", "=#,##0=;"),
                (
                    "1000",
                    "<<$(cardinal,one{ thousand}other{ thousands})$[ >>];",
                ),
            ],
        ),
    ]);
    for (n, want) in [
        (1, "1st"),
        (2, "2nd"),
        (3, "3rd"),
        (4, "4th"),
        (11, "11th"),
        (21, "21st"),
        (112, "112th"),
    ] {
        assert_eq!(int(&r, "%o", n), want);
    }
    assert_eq!(int(&r, "%c", 1000), "1 thousand");
    assert_eq!(int(&r, "%c", 2500), "2 thousands 500");
}

#[test]
fn output_is_postprocessed() {
    let r = rbnf(&[(
        "%x",
        &[
            ("0", "ein\u{ad}hundert  und\u{ad} \t zwei;"),
            ("1", "' and =#,##0=;"),
        ],
    )]);
    assert_eq!(int(&r, "%x", 0), "einhundert und zwei");
    assert_eq!(int(&r, "%x", 1), "and 1");
}

#[test]
fn lenient_parse_ruleset_is_ignored() {
    let r = rbnf(&[
        (
            "%%lenient-parse",
            &[("0", "&[last primary ignorable ] << ' ' << ',';")],
        ),
        ("%x", &[("0", "=#,##0=;")]),
    ]);
    assert_eq!(int(&r, "%x", 3), "3");
}

#[test]
fn nothing_to_say_is_none() {
    let r = rbnf(&[("%x", &[("5", "five;")])]);
    assert_eq!(r.format("%nope", Value::int(5)), None);
    assert_eq!(
        r.format("%x", Value::int(3)),
        None,
        "below the lowest base value"
    );
    assert_eq!(
        r.format("%x", Value::decimal(false, 5, "a")),
        None,
        "non-digit fraction"
    );
    let looping = rbnf(&[("%x", &[("0", "=%x=;")])]);
    assert_eq!(looping.format("%x", Value::int(0)), None, "recursion limit");
}

#[test]
fn unsupported_syntax_is_rejected_with_context() {
    let cases: &[(&str, &str, &str)] = &[
        ("0", "==;", "== is not a legal token"),
        ("0", "=%nope=;", "no ruleset named %nope"),
        ("0", "<0<;", "only supported in =…="),
        ("-x", "<<;", "<< is not allowed"),
        ("x.x", ">%%other>;", "fraction rulesets are not supported"),
        ("0", "<< >> =%%other=;", "more than two substitutions"),
        ("0", "a [b;", "unmatched bracket"),
        ("0", "a ]b[;", "unmatched bracket"),
        ("0", "[a][b];", "nested or repeated brackets"),
        ("-x", "a [b];", "brackets are not allowed"),
        ("0", "a; b;", "only one rule per body"),
        ("foo", "a;", "unknown rule descriptor"),
        ("1x", "a;", "unknown rule descriptor"),
        ("10/0", "a;", "radix must be at least 2"),
        ("10>5", "a;", "unexpected '5'"),
        ("0", "$(ordinal,one{st})$;", "without an other{…} form"),
        ("0", "$(plural,other{s})$;", "unknown plural type"),
        ("0", "$(cardinal,other{s};", "unterminated $( plural block"),
        ("0", "=#,##,##0=;", "secondary grouping"),
        ("0", "=#.#0=;", "unsupported decimal-format pattern"),
        ("0", "a < b;", "unmatched '<'"),
        ("0", "5 $;", "stray $"),
    ];
    for (descriptor, body, message) in cases {
        let err = compile(
            ".",
            &[("%%other", &[("0", "o;")]), ("%x", &[(descriptor, body)])],
        )
        .err()
        .unwrap_or_else(|| panic!("{descriptor}: {body} was accepted"));
        assert_eq!(err.ruleset, "%x", "{descriptor}: {body}");
        assert_eq!(err.rule, format!("{descriptor}: {body}"));
        assert!(
            err.message.contains(message),
            "{descriptor}: {body} -> {}",
            err.message
        );
    }

    let out_of_order = compile(".", &[("%x", &[("10", "a;"), ("5", "b;")])])
        .err()
        .unwrap();
    assert!(
        out_of_order.message.contains("not in order"),
        "{out_of_order}"
    );
    let no_predecessor = compile(".", &[("%x", &[("10", ">>>;")])]).err().unwrap();
    assert!(
        no_predecessor.message.contains("no preceding rule"),
        "{no_predecessor}"
    );
    let no_public = compile(".", &[("%%x", &[("0", "a;")])]).err().unwrap();
    assert!(
        no_public.message.contains("no public ruleset"),
        "{no_public}"
    );
    let duplicate = compile(".", &[("%x", &[("0", "a;")]), ("%x", &[("0", "b;")])])
        .err()
        .unwrap();
    assert!(
        duplicate.message.contains("duplicate ruleset"),
        "{duplicate}"
    );
}

#[test]
fn every_vendored_language_compiles() {
    for language in crate::Language::ALL {
        Rbnf::new(language.data()).unwrap_or_else(|e| panic!("{}: {e}", language.code()));
    }
}

#[test]
fn compiled_rulesets_are_shareable() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Rbnf>();
}
