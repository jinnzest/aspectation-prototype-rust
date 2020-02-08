use aspects::model::*;
use semantic::model::FunctionSignature;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct SideEffectAspect {}

impl SideEffectAspect {
    pub fn name() -> String {
        "side_effect".to_owned()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SideEffectAnalytics {
    pub values: HashSet<SideEffectAnalyticsValue>,
}

impl Analytics for SideEffectAnalytics {}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum SideEffectAnalyticsValue {
    ConsoleOutput,
    ConsoleInput,
    None,
}

impl fmt::Display for SideEffectAnalytics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vec = self
            .values
            .iter()
            .collect::<Vec<&SideEffectAnalyticsValue>>();
        vec.sort();
        let res = vec.iter().fold("".to_owned(), |acc, a| {
            if acc.is_empty() {
                if **a == SideEffectAnalyticsValue::None {
                    format!("{}", a)
                } else {
                    format!("allowed side effects: {}", a)
                }
            } else {
                format!("{}, {}", acc, a)
            }
        });
        write!(f, "{}", res)
    }
}

impl fmt::Display for SideEffectAnalyticsValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SideEffectAnalyticsValue::ConsoleOutput => write!(f, "console output"),
            SideEffectAnalyticsValue::ConsoleInput => write!(f, "console input"),
            SideEffectAnalyticsValue::None => write!(f, "no side effects"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SideEffectHint {
    NoSideEffects,
    AnySideEffect,
    AllowedSideEffects(SideEffectAnalytics),
}

impl fmt::Display for SideEffectHint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SideEffectHint::NoSideEffects => write!(f, "none"),
            SideEffectHint::AnySideEffect => write!(f, "any"),
            SideEffectHint::AllowedSideEffects(analytics) => {
                let values_str = analytics.values.iter().fold("".to_owned(), |acc, v| {
                    if acc.is_empty() {
                        format!("{}", v)
                    } else {
                        format!("{}, {}", acc, v)
                    }
                });
                write!(f, "{}", values_str)
            }
        }
    }
}

impl Hint for SideEffectHint {}

#[derive(Debug, Clone)]
pub struct FnWithSideEffectAnalytics {
    pub sig: Rc<FunctionSignature>,
    pub analytics: Rc<SideEffectAnalytics>,
}

#[cfg(test)]
mod test_side_effect_model {
    use aspects::side_effect::model::{SideEffectAnalytics, SideEffectAnalyticsValue};
    use std::collections::HashSet;

    #[test]
    pub fn print_analytics_side_effects_for_console_input_and_output() {
        let mut values = HashSet::new();
        values.insert(SideEffectAnalyticsValue::ConsoleInput);
        values.insert(SideEffectAnalyticsValue::ConsoleOutput);
        assert_eq!(
            format!("{}", SideEffectAnalytics { values }),
            "allowed side effects: console output, console input"
        );
    }
}
