use semantic::model::{Construction, FuncName, Function};
use std::collections::HashMap;
use std::fmt::Display;

pub fn extract_functions(constructions: &[Construction]) -> HashMap<FuncName, &Function> {
    constructions
        .iter()
        .filter_map(|c| match c {
            Construction::Function(f) => Some(f),
            _ => None,
        })
        .map(|f| (f.name.clone(), f))
        .collect()
}

pub fn str_by_comma<T: Display>(items: &[T]) -> String {
    items.iter().fold("".to_owned(), |acc, v| {
        if acc.is_empty() {
            format!("{}", v)
        } else {
            format!("{}, {}", acc, v)
        }
    })
}
