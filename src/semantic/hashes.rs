extern crate crypto;

use self::crypto::digest::Digest;
use self::crypto::sha2::Sha256;
use aspects::utils::{get_func, is_function};
use files::*;
use model::*;
use parsing::model::{Error, Errors, Loc};
use paths::hashes_file;
use semantic::model::*;
use std::collections::HashMap;
use std::collections::HashSet;
use utils::*;

pub fn write_hashes<S: ::std::hash::BuildHasher>(
    source_file_name: &str,
    hashes: &HashMap<FuncName, SemanticHash, S>,
) {
    write_to_file(&hashes_file(source_file_name), || hashes_to_str(hashes));
}

fn hashes_to_str<S: ::std::hash::BuildHasher>(
    hashes: &HashMap<FuncName, SemanticHash, S>,
) -> String {
    let mut tuples: Vec<(FuncName, SemanticHash)> = hashes
        .iter()
        .map(|(f, sh)| (f.clone(), sh.clone()))
        .collect();
    tuples.sort_by(|(name1, _), (name2, _)| name1.partial_cmp(name2).unwrap());
    tuples.iter().fold("".to_owned(), |acc, (name, hash)| {
        format!("{} = {}\n{}", name, hash, acc)
    })
}

pub fn read_hashes(source_file_name: &str) -> Result<HashMap<SemanticHash, FuncName>, Errors> {
    match read_from_file(&hashes_file(source_file_name), &str_to_hashes) {
        Ok(ok) => Ok(ok),
        Err(mut err) => {
            err.push(Error {
                message: "Reading hashes error:".to_owned(),
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

fn str_to_hashes(body: &str) -> Result<HashMap<SemanticHash, FuncName>, Errors> {
    let mut semantics = HashMap::new();
    body.split('\n')
        .filter(|l| !l.trim().is_empty())
        .try_for_each(|line| parse_hashes_line(&mut semantics, line))?;
    Ok(semantics)
}

fn parse_hashes_line(acc: &mut HashMap<SemanticHash, FuncName>, line: &str) -> Result<(), Errors> {
    let result: Result<NameValue<FuncName, SemanticHash>, Errors> = parse_name_value(
        line,
        "=",
        &|n| Ok(FuncName::new(n)),
        "name of a function",
        &|v| Ok(SemanticHash::new(v)),
        "hash of a function",
    );
    match result {
        Ok(res) => {
            acc.insert(res.value, res.name);
            Ok(())
        }
        Err(err) => Err(err),
    }
}

pub fn generate_hashes(
    constructions: &[Construction],
    internal_funcs: &HashSet<FuncName>,
) -> HashMap<FuncName, SemanticHash> {
    let mut sha = Sha256::new();
    let mut hashes = HashMap::new();
    let source_code_funcs = constructions
        .iter()
        .filter(|c| is_function(c))
        .map(get_func)
        .collect();
    for c in constructions {
        if let Construction::Function(f) = c {
            let mut calling_functions = HashSet::new();
            calling_functions.insert(f.name.clone());
            generate_hash(
                &f,
                &mut sha,
                &mut hashes,
                &source_code_funcs,
                &internal_funcs,
            );
        }
    }
    hashes
}

fn generate_hash(
    f: &Function,
    sha: &mut Sha256,
    hashes: &mut HashMap<FuncName, SemanticHash>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    external_funcs: &HashSet<FuncName>,
) -> SemanticHash {
    let mut hash_source_acc = "".to_owned();
    let hash_source = gen_hash_for_expr(f, &f.body, sha, hashes, source_code_funcs, external_funcs);
    hash_source_acc = format!("{} {}", hash_source_acc, hash_source);
    sha.input_str(&hash_source_acc);
    let hash = SemanticHash::new(&sha.result_str());
    sha.reset();
    hashes.insert(f.name.clone(), hash.clone());
    hash
}

fn gen_hash_for_expr(
    f: &Function,
    expr: &Expression,
    sha: &mut Sha256,
    hashes: &mut HashMap<FuncName, SemanticHash>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    external_funcs: &HashSet<FuncName>,
) -> SemanticHash {
    let args: Vec<String> = f.args.iter().map(|i| i.str().to_owned()).collect();
    use semantic::model::Expression::*;
    match expr {
        Constant(s) => SemanticHash::new(s.str()),
        FunctionCall(sig) => {
            let name = &sig.name;
            if source_code_funcs.contains_key(name) {
                let func = source_code_funcs.get(name).unwrap();
                if hashes.contains_key(name) {
                    hashes.get(name).unwrap().clone()
                } else {
                    generate_hash(&func, sha, hashes, source_code_funcs, external_funcs)
                }
            } else if external_funcs.contains(name) {
                SemanticHash::new(name.str())
            } else if args.contains(&name.str().to_owned()) {
                SemanticHash::new(&format!(
                    "int_arg_{}",
                    args.iter()
                        .position(|arg| arg == name.str())
                        .unwrap_or_default()
                ))
            } else {
                panic!("access to external variables is not supported yet");
            }
        }
        FunctionArgument(name) => {
            if args.contains(&name.str().to_owned()) {
                SemanticHash::new(&format!(
                    "int_arg_{}",
                    args.iter()
                        .position(|arg| arg == name.str())
                        .unwrap_or_default()
                ))
            } else {
                panic!("access to external variables is not supported yet");
            }
        }
        SubExpression(exprs) => SemanticHash::new(
            &exprs
                .iter()
                .map(|e| gen_hash_for_expr(f, e, sha, hashes, source_code_funcs, external_funcs))
                .fold("".to_owned(), |acc, v| format!("{} {}", acc, v)),
        ),
    }
}

pub fn map_old_to_new_names(
    old_hashes: &HashMap<SemanticHash, FuncName>,
    new_hashes: &HashMap<FuncName, SemanticHash>,
) -> HashMap<FuncName, FuncName> {
    let mut hashes = HashMap::new();
    for (name, hash) in new_hashes {
        if old_hashes.contains_key(hash) {
            let old_name = old_hashes.get(hash).unwrap().clone();
            if &old_name != name {
                hashes.insert(old_name, name.clone());
            }
        }
    }
    print_old_to_new_names(&hashes);
    hashes
}

fn print_old_to_new_names(old_to_new_names: &HashMap<FuncName, FuncName>) {
    old_to_new_names
        .iter()
        .for_each(|(o, n)| println!("old_to_new_names, old: {:?}, new: {:?}", o, n));
}

#[cfg(test)]
mod test_hashes {
    use super::*;
    use parsing::model::Ident;

    fn mk_ident(s: &str) -> Ident {
        Ident::new(s)
    }

    #[test]
    fn empty_result_for_empty_input() {
        let result = str_to_hashes("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn empty_result_for_spaces_only_input() {
        let result = str_to_hashes("\t\n\n  \n ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn check_equals_absence() {
        let mut map = HashMap::new();
        let result = parse_hashes_line(&mut map, "a b");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().get(0).unwrap().message,
            "Expected '=' separator but the string doesn't contain it: 'a b'"
        );
    }

    #[test]
    fn check_func_name_absence() {
        let mut map = HashMap::new();
        let result = parse_hashes_line(&mut map, " = b");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().get(0).unwrap().message, "Expected name of a function on left side of '=' but the string doesn't contain it: ' = b'");
    }

    #[test]
    fn check_hash_absence() {
        let mut map = HashMap::new();
        let result = parse_hashes_line(&mut map, "a = ");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().get(0).unwrap().message, "Expected hash of a function on right side of '=' but the string doesn't contain it: 'a = '");
    }

    #[test]
    fn write_hashes_to_str() {
        let mut hashes = HashMap::new();
        hashes.insert(FuncName::new("a"), SemanticHash::new("hash for a"));
        hashes.insert(FuncName::new("b"), SemanticHash::new("hash for b"));
        hashes.insert(FuncName::new("c"), SemanticHash::new("hash for c"));

        let s = hashes_to_str(&mut hashes);
        assert_eq!(s, "a = hash for a\nb = hash for b\nc = hash for c\n");
    }

    #[test]
    fn sort_and_write_hashes_to_str() {
        let mut hashes = HashMap::new();
        hashes.insert(FuncName::new("a"), SemanticHash::new("hash for a"));
        hashes.insert(FuncName::new("b"), SemanticHash::new("hash for b"));
        hashes.insert(FuncName::new("d"), SemanticHash::new("hash for d"));
        hashes.insert(FuncName::new("c"), SemanticHash::new("hash for c"));
        hashes.insert(FuncName::new("e"), SemanticHash::new("hash for e"));

        let s = hashes_to_str(&mut hashes);
        assert_eq!(
            s,
            "a = hash for a\nb = hash for b\nc = hash for c\nd = hash for d\ne = hash for e\n"
        );
    }

    #[test]
    fn read_hashes_from_str() {
        let s = "   a = hash for a   \nb = hash for b\nc = hash for c\nd = hash for d\ne = hash for e\n";
        let hashes = str_to_hashes(s);
        let map = hashes.unwrap();

        assert_eq!(map.len(), 5);
        assert_eq!(
            map.get(&SemanticHash::new("hash for a")).unwrap().str(),
            "a"
        );
        assert_eq!(
            map.get(&SemanticHash::new("hash for b")).unwrap().str(),
            "b"
        );
        assert_eq!(
            map.get(&SemanticHash::new("hash for c")).unwrap().str(),
            "c"
        );
        assert_eq!(
            map.get(&SemanticHash::new("hash for d")).unwrap().str(),
            "d"
        );
        assert_eq!(
            map.get(&SemanticHash::new("hash for e")).unwrap().str(),
            "e"
        );
    }

    #[test]
    fn hashes_for_functions_with_different_names_of_the_same_arg_positions_are_equal() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("arg2"),
                args: vec![],
            }),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg22")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("arg22"),
                args: vec![],
            }),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let source_code_funcs = HashMap::new();
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f1,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f2,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_eq!(hash1.str(), hash2.str());
    }

    #[test]
    fn hashes_for_different_int_consts_are_different() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("2")),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let source_code_funcs = HashMap::new();
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f1,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f2,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_ne!(hash1.str(), hash2.str());
    }

    #[test]
    fn hashes_for_equal_int_consts_are_equal() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let source_code_funcs = HashMap::new();
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f1,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f2,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_eq!(hash1.str(), hash2.str());
    }

    #[test]
    fn hashes_for_different_args_in_body_are_different() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("arg1"),
                args: vec![],
            }),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("arg2"),
                args: vec![],
            }),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let source_code_funcs = HashMap::new();
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f1,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f2,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_ne!(hash1.str(), hash2.str());
    }

    #[test]
    fn hashes_for_functions_calling_different_functions_are_different() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("2")),
        };
        let f3 = Function {
            name: FuncName::new("func3"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("func1"),
                args: vec![],
            }),
        };
        let f4 = Function {
            name: FuncName::new("func4"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("func2"),
                args: vec![],
            }),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(f1.name.clone(), &f1);
        source_code_funcs.insert(f2.name.clone(), &f2);
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f3,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f4,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_ne!(hash1.str(), hash2.str());
    }

    #[test]
    fn hashes_for_functions_calling_the_same_hash_functions_are_equal() {
        let f1 = Function {
            name: FuncName::new("func1"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let f2 = Function {
            name: FuncName::new("func2"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::Constant(Ident::new("1")),
        };
        let f3 = Function {
            name: FuncName::new("func3"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("func1"),
                args: vec![],
            }),
        };
        let f4 = Function {
            name: FuncName::new("func4"),
            args: [mk_ident("arg1"), mk_ident("arg2")].to_vec(),
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("func2"),
                args: vec![],
            }),
        };
        let mut sha = Sha256::new();
        let mut hashes = HashMap::new();
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(f1.name.clone(), &f1);
        source_code_funcs.insert(f2.name.clone(), &f2);
        let internal_funcs_set = HashSet::new();
        let hash1 = generate_hash(
            &f3,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        let hash2 = generate_hash(
            &f4,
            &mut sha,
            &mut hashes,
            &source_code_funcs,
            &internal_funcs_set,
        );
        assert_eq!(hash1.str(), hash2.str());
    }
}
