use aspects::model::Aspect;
use aspects::register::{to_aspect_trait, AspectWrapper};
use parsing::model::Errors;
use semantic::model::*;
use std::collections::HashMap;
use std::rc::Rc;

pub fn remap<T>(
    input: HashMap<FuncName, T>,
    old_to_new_names: &HashMap<FuncName, FuncName>,
) -> HashMap<FuncName, T> {
    input
        .into_iter()
        .map(|(old_name, a)| match old_to_new_names.get(&old_name) {
            Some(new_name) => (new_name.clone(), a),
            None => (old_name, a),
        })
        .collect()
}

pub fn read_all<T>(
    aspects: &[AspectWrapper],
    f: &impl Fn(Rc<dyn Aspect>) -> Result<HashMap<FuncName, T>, Errors>,
) -> Result<HashMap<FuncName, Vec<T>>, Errors> {
    let mut analytics: HashMap<FuncName, Vec<T>> = HashMap::new();
    aspects.iter().try_for_each(|a| {
        let result: Result<HashMap<FuncName, T>, Errors> = f(to_aspect_trait(a));
        match result {
            Ok(result) => {
                result.into_iter().for_each(|(k, v)| {
                    analytics.entry(k).or_insert_with(Vec::new).push(v);
                });
                Ok(())
            }
            Err(err) => Err(err),
        }
    })?;
    Ok(analytics)
}

use std::collections::hash_set::HashSet;

pub fn is_function(c: &Construction) -> bool {
    match c {
        Construction::Function(_) => true,
        _ => false,
    }
}

pub fn is_intersect<S: ::std::hash::BuildHasher>(
    calling_functions: &HashSet<FuncName, S>,
    names: &HashSet<FuncName, S>,
) -> bool {
    calling_functions.intersection(names).count() > 0
}

pub fn get_func(c: &Construction) -> (FuncName, &Function) {
    match c {
        Construction::Function(f) => (f.name.clone(), f),
        _ => panic!(),
    }
}
