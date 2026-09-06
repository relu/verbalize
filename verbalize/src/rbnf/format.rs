//! Rule evaluation. Follows ICU4J `NFRuleSet.findRule`/`findNormalRule`,
//! `NFRule.doFormat` and the `NFSubstitution` subclasses' `transformNumber`,
//! in integer arithmetic on [`Value`].

use super::{Piece, Rbnf, Rule, RuleSet, SubKind, Target, Value};

/// ICU's `RECURSION_LIMIT`: a ruleset that keeps delegating is broken data.
const RECURSION_LIMIT: u32 = 64;

/// The rules have no answer: recursion ran away, or a value fell below the
/// lowest base value of a ruleset that has no rule for it.
pub(super) struct Unformattable;

type Result<T> = std::result::Result<T, Unformattable>;

impl Rbnf {
    pub(super) fn format_value(
        &self,
        index: usize,
        v: Value<'_>,
        out: &mut String,
        depth: u32,
    ) -> Result<()> {
        if depth >= RECURSION_LIMIT {
            return Err(Unformattable);
        }
        let set = &self.rulesets[index];
        let rule = set.find_rule(v)?;
        self.apply(set, rule, v, out, depth + 1)
    }

    fn apply(
        &self,
        set: &RuleSet,
        rule: &Rule,
        v: Value<'_>,
        out: &mut String,
        depth: u32,
    ) -> Result<()> {
        for piece in &rule.pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Plural(plural) => {
                    // ICU: the plural operand is the number of divisors,
                    // or the rounded value inside a fraction rule.
                    let operand = if v.is_fraction() && v.integer == 0 {
                        round_half_up(v)
                    } else {
                        v.integer / rule.divisor
                    };
                    let rules = if plural.ordinal {
                        &self.ordinal
                    } else {
                        &self.cardinal
                    };
                    let category = rules
                        .as_ref()
                        .expect("plural rules are built when a rule needs them")
                        .category_for(u64::try_from(operand).unwrap_or(u64::MAX));
                    let form = plural
                        .forms
                        .iter()
                        .find(|(c, _)| *c == category)
                        .or_else(|| {
                            plural
                                .forms
                                .iter()
                                .find(|(c, _)| *c == icu_plurals::PluralCategory::Other)
                        })
                        .expect("parser requires an other{} form");
                    out.push_str(&form.1);
                }
                Piece::Sub(sub) => {
                    // Without a -x rule the value keeps its sign here, and
                    // ICU's Java arithmetic truncates toward zero: both
                    // quotient and remainder stay negative.
                    let value = match sub.kind {
                        SubKind::Multiplier => {
                            Value::decimal(v.negative, v.integer / rule.divisor, "")
                        }
                        SubKind::Modulus => {
                            Value::decimal(v.negative, v.integer % rule.divisor, v.fraction)
                        }
                        SubKind::Predecessor(index) => {
                            let modulus =
                                Value::decimal(v.negative, v.integer % rule.divisor, v.fraction);
                            self.apply(set, &set.rules[index], modulus, out, depth)?;
                            continue;
                        }
                        SubKind::SameValue => v,
                        // ICU floors here, turning -2.5 into -3; nothing in
                        // CLDR reaches a fraction rule with a signed value.
                        SubKind::IntegralPart => Value::decimal(v.negative, v.integer, ""),
                        SubKind::FractionalPart { spaces } => {
                            let Target::RuleSet(index) = sub.target else {
                                unreachable!("parser only allows the own ruleset here");
                            };
                            for (i, digit) in v.fraction.bytes().enumerate() {
                                if i > 0 && spaces {
                                    out.push(' ');
                                }
                                let digit = Value::int(i128::from(digit - b'0'));
                                self.format_value(index, digit, out, depth)?;
                            }
                            continue;
                        }
                        SubKind::AbsoluteValue => Value::decimal(false, v.integer, v.fraction),
                    };
                    match &sub.target {
                        Target::RuleSet(index) => self.format_value(*index, value, out, depth)?,
                        Target::Decimal(pattern) => pattern.format(value, &self.symbols, out),
                    }
                }
            }
        }
        Ok(())
    }
}

impl RuleSet {
    /// `NFRuleSet.findRule`: the negative rule for negative values, the
    /// fraction rules for fractional values, the master rule when present,
    /// otherwise the normal rules by base value. Without a negative rule the
    /// magnitude selects the rule but the rule is applied to the signed
    /// value, so a delegating `=%other=` still reaches the other ruleset's
    /// negative rule.
    fn find_rule(&self, v: Value<'_>) -> Result<&Rule> {
        if v.negative {
            if let Some(rule) = &self.negative {
                return Ok(rule);
            }
        }
        if !v.is_fraction() {
            self.normal_rule(v.integer)
        } else if let (Some(proper), true) = (&self.proper, v.integer == 0) {
            Ok(proper)
        } else if let Some(improper) = &self.improper {
            Ok(improper)
        } else if let Some(master) = &self.master {
            Ok(master)
        } else {
            self.normal_rule(round_half_up(v))
        }
    }

    /// `NFRuleSet.findNormalRule`: the rule with the largest base value not
    /// above `n`, one rule back when the rollback rule applies.
    fn normal_rule(&self, n: u128) -> Result<&Rule> {
        if self.rules.is_empty() {
            return self.master.as_ref().ok_or(Unformattable);
        }
        let hi = self.rules.partition_point(|r| r.base <= n);
        let rule = self.rules.get(hi.wrapping_sub(1)).ok_or(Unformattable)?;
        if rule.should_roll_back(n) {
            return self.rules.get(hi.wrapping_sub(2)).ok_or(Unformattable);
        }
        Ok(rule)
    }
}

impl Rule {
    /// `NFRule.shouldRollBack`: `100: << hundred[ >>];` expands to rules at
    /// 100 and 101; 200 must use the one at 100, not "two hundred zero".
    fn should_roll_back(&self, n: u128) -> bool {
        self.has_modulus
            && n.is_multiple_of(self.divisor)
            && !self.base.is_multiple_of(self.divisor)
    }
}

/// `Math.round` on a digit string: up when the first fraction digit is 5+.
fn round_half_up(v: Value<'_>) -> u128 {
    match v.fraction.as_bytes().first() {
        Some(d) if *d >= b'5' => v.integer.saturating_add(1),
        _ => v.integer,
    }
}
