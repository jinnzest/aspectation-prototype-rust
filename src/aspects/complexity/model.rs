use aspects::model::*;
use parsing::model::Ident;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ComplexityAspect {}

impl ComplexityAspect {
    pub fn name() -> String {
        "complexity".to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum ComplexityAnalyticsValue {
    OC,
    ON,
    ON2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityAnalytics {
    pub values: HashMap<Ident, ComplexityAnalyticsValue>,
}

impl ComplexityAnalytics {
    pub fn new() -> Rc<Self> {
        Rc::new(ComplexityAnalytics {
            values: HashMap::new(),
        })
    }
}

impl Analytics for ComplexityAnalytics {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityHint {
    pub values: HashMap<Ident, ComplexityHintValue>,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum ComplexityHintValue {
    OC,
    ON,
    Any,
}

impl Hint for ComplexityHint {}

impl fmt::Display for ComplexityAnalytics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vec = self
            .values
            .iter()
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect::<Vec<(Ident, ComplexityAnalyticsValue)>>();
        vec.sort();
        let values_str = vec.into_iter().fold("".to_owned(), |acc, (name, value)| {
            if acc.is_empty() {
                format!("{} is {}", name, value)
            } else {
                format!("{}, {} is {}", acc, name, value)
            }
        });
        write!(f, "{}", values_str)
    }
}

impl fmt::Display for ComplexityAnalyticsValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ComplexityAnalyticsValue::OC => write!(f, "O(c)"),
            ComplexityAnalyticsValue::ON => write!(f, "O(n)"),
            ComplexityAnalyticsValue::ON2 => write!(f, "O(n^2)"),
        }
    }
}

impl fmt::Display for ComplexityHint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vec = self
            .values
            .iter()
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect::<Vec<(Ident, ComplexityHintValue)>>();
        vec.sort();
        let values_str = vec.into_iter().fold("".to_owned(), |acc, (name, value)| {
            if acc.is_empty() {
                format!("{}: {}", name, value)
            } else {
                format!("{}, {}: {}", acc, name, value)
            }
        });
        write!(f, "{}", values_str)
    }
}

impl fmt::Display for ComplexityHintValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ComplexityHintValue::OC => write!(f, "c"),
            ComplexityHintValue::ON => write!(f, "n"),
            ComplexityHintValue::Any => write!(f, "any"),
        }
    }
}

#[cfg(test)]
mod test_complexity_model {
    use aspects::complexity::model::{
        ComplexityAnalytics, ComplexityAnalyticsValue, ComplexityHint, ComplexityHintValue,
    };
    use parsing::model::Ident;
    use std::collections::HashMap;

    #[test]
    pub fn print_analytics_complexity_for_on_and_oc() {
        let mut values = HashMap::new();
        values.insert(Ident::new("arg1"), ComplexityAnalyticsValue::ON);
        values.insert(Ident::new("arg2"), ComplexityAnalyticsValue::OC);
        assert_eq!(
            format!("{}", ComplexityAnalytics { values }),
            "arg1 is O(n), arg2 is O(c)"
        );
    }

    #[test]
    pub fn print_hints_complexity_for_on_and_oc() {
        let mut values = HashMap::new();
        values.insert(Ident::new("arg1"), ComplexityHintValue::ON);
        values.insert(Ident::new("arg2"), ComplexityHintValue::OC);
        assert_eq!(format!("{}", ComplexityHint { values }), "arg1: n, arg2: c");
    }
}
