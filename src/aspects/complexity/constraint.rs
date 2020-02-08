use aspects::analytics::extract_analytics_from_wrapper;
use aspects::complexity::analytics::{filter_complexity_analytics, filter_complexity_hint};
use aspects::complexity::model::{ComplexityAnalyticsValue, ComplexityHintValue};
use aspects::hints::extract_hints_from_wrapper;
use aspects::register::{AnalyticsWrapper, HintWrapper};
use parsing::model::Ident;
use semantic::utils::str_by_comma;
use std::collections::HashMap;

pub fn check_complexity_constraint(
    hints: &[HintWrapper],
    analytics: &[AnalyticsWrapper],
) -> String {
    let hint = extract_hints_from_wrapper(hints, &filter_complexity_hint);
    let analytics_item = extract_analytics_from_wrapper(analytics, &filter_complexity_analytics);
    match hint {
        Some(hrc) => match analytics_item {
            Some(arc) => {
                let mut h_values = clone_tuples(&hrc.values);
                let mut a_values = clone_tuples(&arc.values);
                sort_by_ident(&mut h_values);
                sort_by_ident(&mut a_values);
                let results: Vec<String> = h_values
                    .iter()
                    .zip(&a_values)
                    .filter_map(|((_, h), (i, a))| match_hint_to_analytics(h, i, a))
                    .collect();
                str_by_comma(&results)
            }
            _ => "".to_owned(),
        },
        None => "".to_owned(),
    }
}

fn sort_by_ident<S>(values: &mut Vec<(Ident, S)>) {
    values.sort_by(|(l, _), (r, _)| l.cmp(r))
}

fn match_hint_to_analytics(
    h: &ComplexityHintValue,
    i: &Ident,
    a: &ComplexityAnalyticsValue,
) -> Option<String> {
    match h {
        ComplexityHintValue::OC => {
            if a == &ComplexityAnalyticsValue::OC {
                None
            } else {
                Some(format!(
                    "Maximum \'{}\' is allowed for argument \'{}\' but got \'{}\'",
                    ComplexityAnalyticsValue::OC,
                    i,
                    a
                ))
            }
        }
        ComplexityHintValue::ON => {
            if a == &ComplexityAnalyticsValue::ON2 {
                Some(format!(
                    "Maximum \'{}\' is allowed for argument \'{}\' but got \'{}\'",
                    ComplexityAnalyticsValue::ON,
                    i,
                    a
                ))
            } else {
                None
            }
        }
        ComplexityHintValue::Any => None,
    }
}

fn clone_tuples<F: Clone, S: Clone>(values: &HashMap<F, S>) -> Vec<(F, S)> {
    values.iter().map(|(i, v)| (i.clone(), v.clone())).collect()
}

#[cfg(test)]
mod complexity_constraint {
    use aspects::complexity::constraint::check_complexity_constraint;
    use aspects::complexity::model::{
        ComplexityAnalytics, ComplexityAnalyticsValue, ComplexityHint, ComplexityHintValue,
    };
    use aspects::register::{AnalyticsWrapper, HintWrapper};
    use parsing::model::Ident;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn hint_and_analytics_equal() {
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityHintValue::ON);
        let hints = vec![HintWrapper::Complexity(Rc::new(ComplexityHint {
            values: values.clone(),
        }))];
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityAnalyticsValue::ON);
        let analytics = vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
            values,
        }))];
        let result = check_complexity_constraint(&hints, &analytics);
        assert_eq!(result, "");
    }

    #[test]
    fn hint_on_and_analytics_oc() {
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityHintValue::ON);
        let hints = vec![HintWrapper::Complexity(Rc::new(ComplexityHint {
            values: values.clone(),
        }))];
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityAnalyticsValue::OC);
        let analytics = vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
            values,
        }))];
        let result = check_complexity_constraint(&hints, &analytics);
        assert_eq!(result, "");
    }

    #[test]
    fn hint_oc_but_analytics_on() {
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityHintValue::OC);
        let hints = vec![HintWrapper::Complexity(Rc::new(ComplexityHint {
            values: values.clone(),
        }))];
        let mut values = HashMap::new();
        values.insert(Ident::new("arg"), ComplexityAnalyticsValue::ON);
        let analytics = vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
            values,
        }))];
        let result = check_complexity_constraint(&hints, &analytics);
        assert_eq!(
            result,
            "Maximum 'O(c)' is allowed for argument 'arg' but got 'O(n)'"
        );
    }
}
