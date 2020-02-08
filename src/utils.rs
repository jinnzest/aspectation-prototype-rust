use model::NameValue;
use parsing::model::Errors;
use parsing::model::{Error, Loc};
use paths::*;
use semantic::model::{Construction, Expression, FuncName};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::Path;

pub fn stop_on_recursion(constructions: &[Construction]) -> Result<(), Errors> {
    let functions = constructions
        .iter()
        .filter_map(|c| match c {
            Construction::Function(f) => Some((f.name.clone(), f.body.clone())),
            _ => None,
        })
        .collect::<HashMap<FuncName, Expression>>();
    let result = functions.iter().try_for_each(|(n, b)| {
        let mut calling_functions = HashSet::new();
        calling_functions.insert(n.clone());
        stop_on_recursion_func(b, &functions, &calling_functions)
    });
    match result {
        Ok(_) => Ok(()),
        Err(mut err) => {
            err.push(Error {
                message:
                    "recursive functions are not supported yet but those functions are recursive:"
                        .to_owned(),
                loc: Loc {
                    pos: 0,
                    line: 0,
                    col: 0,
                },
            });
            Err(err)
        }
    }
}

fn stop_on_recursion_func(
    body: &Expression,
    functions: &HashMap<FuncName, Expression>,
    calling_functions: &HashSet<FuncName>,
) -> Result<(), Errors> {
    match body {
        Expression::FunctionCall(sig) => {
            if calling_functions.contains(&sig.name) {
                Err(vec![Error {
                    message: sig.name.to_owned(),
                    loc: Loc {
                        pos: 0,
                        line: 0,
                        col: 0,
                    },
                }])
            } else if functions.contains_key(&sig.name) {
                let mut calling_functions_copy = calling_functions.clone();
                calling_functions_copy.insert(sig.name.clone());
                let b = functions.get(&sig.name).unwrap();
                stop_on_recursion_func(b, &functions, &calling_functions_copy)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

pub fn create_all_paths() {
    create_dir_if_absent(&project_path());
    create_dir_if_absent(&hashes_path());
    create_dir_if_absent(&hints_path());
    create_dir_if_absent(&analytics_path());
}

pub fn remove_all_paths() {
    remove_dir(&project_path());
    remove_dir(&hashes_path());
    remove_dir(&hints_path());
    remove_dir(&analytics_path());
}

fn remove_dir(path: &str) {
    if Path::new(path).exists() {
        if let Err(err) = std::fs::remove_dir_all(path) {
            if err.raw_os_error().is_some() && err.raw_os_error().unwrap() != 2 {
                println!(
                    "test dir '{}' can't be deleted because of error: {}",
                    path, err
                )
            }
        }
    }
}

fn create_dir_if_absent(path: &str) {
    if !Path::new(path).exists() {
        match std::fs::create_dir_all(path) {
            Result::Err(err) => match err.kind() {
                ErrorKind::AlreadyExists => {}
                _ => {
                    println!("{:?}", err);
                    panic!("can't create dir \"{}\"", path);
                }
            },
            Result::Ok(_) => {
                //                println!("'{}' dir has been created", path)
            }
        };
    }
}

pub fn parse_name_value<N, V>(
    name_value: &str,
    sep: &str,
    name_parser: &impl Fn(&str) -> Result<N, Errors>,
    name: &str,
    value_parser: &impl Fn(&str) -> Result<V, Errors>,
    value: &str,
) -> Result<NameValue<N, V>, Errors> {
    let values = name_value.split(sep).collect::<Vec<&str>>();
    wrap_result(validate(name_value, &values, name, value, sep))?;
    let name = name_parser(values[0].trim())?;
    let value = value_parser(values[1].trim())?;
    Ok(NameValue { name, value })
}

fn validate(
    name_value: &str,
    values: &[&str],
    name: &str,
    value: &str,
    sep: &str,
) -> Result<(), String> {
    if values.len() < 2 {
        Err(format!(
            "Expected '{}' separator but the string doesn't contain it: '{}'",
            sep, name_value
        ))
    } else if values[0].trim().is_empty() {
        Err(format!(
            "Expected {} on left side of '{}' but the string doesn't contain it: '{}'",
            name, sep, name_value
        ))
    } else if values[1].trim().is_empty() {
        Err(format!(
            "Expected {} on right side of '{}' but the string doesn't contain it: '{}'",
            value, sep, name_value
        ))
    } else {
        Ok(())
    }
}

fn wrap_result(res: Result<(), String>) -> Result<(), Errors> {
    match res {
        Ok(_) => Ok(()),
        Err(err) => Err(vec![Error {
            message: err,
            loc: Loc {
                pos: 0,
                line: 0,
                col: 0,
            },
        }]),
    }
}

pub fn add_to_errors(err: &str, errors: &mut Errors) {
    errors.push(Error {
        message: err.to_owned(),
        loc: Loc {
            pos: 0,
            line: 0,
            col: 0,
        },
    });
}

#[cfg(test)]
mod test_utils {
    use super::*;
    use semantic::model::{Construction, Expression, Function, FunctionCallSignature};

    #[test]
    pub fn error_on_recursion_test() {
        let result = stop_on_recursion(&vec![Construction::Function(Function {
            name: FuncName::new("a"),
            args: Vec::new(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("a"),
                args: vec![],
            }),
        })]);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err.get(1).unwrap().message,
            "recursive functions are not supported yet but those functions are recursive:"
        );
        assert_eq!(err.get(0).unwrap().message, "a");
    }

    #[test]
    pub fn no_error_without_recursion() {
        let result = stop_on_recursion(&vec![
            Construction::Function(Function {
                name: FuncName::new("a"),
                args: Vec::new(),
                body: Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("b"),
                    args: vec![],
                }),
            }),
            Construction::Function(Function {
                name: FuncName::new("b"),
                args: Vec::new(),
                body: Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("c"),
                    args: vec![],
                }),
            }),
        ]);
        assert!(result.is_ok());
    }
}
