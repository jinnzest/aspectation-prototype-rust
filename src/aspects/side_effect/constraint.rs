use aspects::analytics::extract_analytics_from_wrapper;
use aspects::hints::extract_hints_from_wrapper;
use aspects::register::{AnalyticsWrapper, HintWrapper};
use aspects::side_effect::analytics::{filter_side_effect_analytics, filter_side_effect_hint};
use aspects::side_effect::model::{SideEffectAnalyticsValue, SideEffectHint};

pub fn side_effect_constraint(hints: &[HintWrapper], analytics: &[AnalyticsWrapper]) -> String {
    let hint = extract_hints_from_wrapper(hints, &filter_side_effect_hint);
    let analytics_item = extract_analytics_from_wrapper(analytics, &filter_side_effect_analytics);
    match hint {
        Some(h) => match analytics_item {
            Some(a) => match *h.clone() {
                SideEffectHint::NoSideEffects
                    if a.values.len() > 1
                        || a.values.len() == 1
                            && !a.values.contains(&SideEffectAnalyticsValue::None) =>
                {
                    format!(
                        "Expected: \'{}\'\nGot: \'{}\'",
                        SideEffectAnalyticsValue::None,
                        a
                    )
                }
                SideEffectHint::AllowedSideEffects(ref se) if se.values != a.values => {
                    format!("Expected: \'{}\'\nGot: \'{}\'", se, a)
                }
                _ => "".to_owned(),
            },
            _ => "".to_owned(),
        },
        None => "".to_owned(),
    }
}

#[cfg(test)]
mod side_effect_constraint {
    use aspects::register::{AnalyticsWrapper, HintWrapper};
    use aspects::side_effect::constraint::side_effect_constraint;
    use aspects::side_effect::model::{
        SideEffectAnalytics, SideEffectAnalyticsValue, SideEffectHint,
    };
    use std::collections::HashSet;
    use std::rc::Rc;

    #[test]
    fn hint_and_analytics_equal() {
        let mut values = HashSet::new();
        values.insert(SideEffectAnalyticsValue::ConsoleInput);
        let hints = vec![HintWrapper::SideEffect(Rc::new(
            SideEffectHint::AllowedSideEffects(SideEffectAnalytics {
                values: values.clone(),
            }),
        ))];
        let analytics = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values,
        }))];
        let result = side_effect_constraint(&hints, &analytics);
        assert_eq!(result, "");
    }

    #[test]
    fn hint_any_analytics_console_input() {
        let mut values = HashSet::new();
        values.insert(SideEffectAnalyticsValue::ConsoleInput);
        let hints = vec![HintWrapper::SideEffect(Rc::new(
            SideEffectHint::AnySideEffect,
        ))];
        let analytics = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values,
        }))];
        let result = side_effect_constraint(&hints, &analytics);
        assert_eq!(result, "");
    }

    #[test]
    fn hint_and_analytics_not_equal() {
        let mut values = HashSet::new();
        values.insert(SideEffectAnalyticsValue::ConsoleOutput);
        let hints = vec![HintWrapper::SideEffect(Rc::new(
            SideEffectHint::AllowedSideEffects(SideEffectAnalytics { values }),
        ))];
        let mut values = HashSet::new();
        values.insert(SideEffectAnalyticsValue::ConsoleInput);
        let analytics = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values: values,
        }))];
        let result = side_effect_constraint(&hints, &analytics);
        assert_eq!(result, "Expected: \'allowed side effects: console output\'\nGot: \'allowed side effects: console input\'");
    }

    #[test]
    fn hint_no_side_effects_analytics_console_input() {
        let mut analytics_values = HashSet::new();
        analytics_values.insert(SideEffectAnalyticsValue::ConsoleInput);
        let hints = vec![HintWrapper::SideEffect(Rc::new(
            SideEffectHint::NoSideEffects,
        ))];
        let analytics = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values: analytics_values,
        }))];
        let result = side_effect_constraint(&hints, &analytics);
        assert_eq!(
            result,
            "Expected: \'no side effects\'\nGot: \'allowed side effects: console input\'"
        );
    }
}
