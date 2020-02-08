use aspects::analytics::*;
use aspects::complexity::model::*;
use aspects::register::{AnalyticsWrapper, HintWrapper};
use parsing::model::{Error, Errors, Ident, Loc};
use parsing::tokenizer::{PosTok, Tok, Tokenizer};
use parsing::utils::{
    consume, done, expected_msg, read_ident, shift_err, spaces, while_not_done_or_eof, State,
};
use semantic::model::{
    Expression, FnWithAnalytics, FuncName, Function, FunctionCallSignature, FunctionSignature,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

pub fn generate_complexity_analytics(
    func: &Function,
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    source_code_funcs: &HashMap<FuncName, &Function>,
) {
    let mut concrete_analytics =
        extract_concrete_analytics(analytics, &filter_complexity_analytics);
    let functions_sig = extract_signatures(analytics);
    generate_complexity_analytics_concrete(
        func,
        &mut concrete_analytics,
        source_code_funcs,
        &functions_sig,
    );
    merge_analytics(
        analytics,
        source_code_funcs,
        &mut concrete_analytics,
        &wrapper,
        &matcher_wrapper,
    );
}

pub fn read_complexity_analytics() -> Result<HashMap<FuncName, ComplexityAnalytics>, Errors> {
    read_analytics_from_file(&ComplexityAspect::name(), &analytics_parser)
}

pub fn filter_complexity_analytics(aw: &AnalyticsWrapper) -> Option<Rc<ComplexityAnalytics>> {
    match aw {
        AnalyticsWrapper::Complexity(a) => Some(a.clone()),
        _ => None,
    }
}

pub fn filter_complexity_hint(aw: &HintWrapper) -> Option<Rc<ComplexityHint>> {
    match aw {
        HintWrapper::Complexity(a) => Some(a.clone()),
        _ => None,
    }
}

fn extract_signatures(
    analytics: &HashMap<FuncName, FnWithAnalytics>,
) -> HashMap<FuncName, Rc<FunctionSignature>> {
    analytics
        .iter()
        .map(|(n, a)| (n.clone(), a.sig.clone()))
        .collect()
}

fn matcher_wrapper(aw: &AnalyticsWrapper, a: Rc<ComplexityAnalytics>) -> AnalyticsWrapper {
    match aw {
        AnalyticsWrapper::Complexity(_) => AnalyticsWrapper::Complexity(a),
        other => other.clone(),
    }
}

fn wrapper(a: &Rc<ComplexityAnalytics>) -> AnalyticsWrapper {
    AnalyticsWrapper::Complexity(a.clone())
}

fn generate_complexity_analytics_concrete(
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
) {
    let values =
        gen_complexity_analytics_for_expr(&f.body, &f, analytics, source_code_funcs, signatures);
    merge_fn_analytics(f, analytics, values);
}

fn gen_complexity_analytics_for_expr(
    expr: &Expression,
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    match expr {
        Expression::FunctionCall(sig) => {
            gen_for_function_call(f, analytics, source_code_funcs, signatures, &sig)
        }
        Expression::SubExpression(exprs) => {
            gen_for_sub_expr(&exprs, f, analytics, source_code_funcs, signatures)
        }
        Expression::Constant(_) => gen_for_constant(f),
        Expression::FunctionArgument(arg) => gen_for_arg(f, &arg),
    }
}

fn gen_for_sub_expr(
    exprs: &[Expression],
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    merge_analytics_maps(
        exprs
            .iter()
            .map(|e| {
                gen_complexity_analytics_for_expr(e, f, analytics, source_code_funcs, signatures)
            })
            .collect(),
    )
}

fn merge_analytics_maps(
    maps: Vec<HashMap<Ident, ComplexityAnalyticsValue>>,
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    maps.into_iter().fold(HashMap::new(), |acc, cav| {
        cav.into_iter()
            .map(|(i, c)| match acc.get(&i) {
                Some(v) => {
                    if v.cmp(&c) == Ordering::Less {
                        (i, c)
                    } else {
                        (i, v.clone())
                    }
                }
                None => (i, c),
            })
            .collect()
    })
}

fn gen_for_arg(f: &Function, arg: &Ident) -> HashMap<Ident, ComplexityAnalyticsValue> {
    f.args
        .iter()
        .filter_map(|a| {
            if a == arg {
                Some((arg.clone(), ComplexityAnalyticsValue::OC))
            } else {
                None
            }
        })
        .collect()
}

fn gen_for_constant(f: &Function) -> HashMap<Ident, ComplexityAnalyticsValue> {
    f.args
        .iter()
        .map(|arg| (arg.clone(), ComplexityAnalyticsValue::OC))
        .collect()
}

fn gen_for_function_call(
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
    sig: &FunctionCallSignature,
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    gen_for_sub_func(analytics, &source_code_funcs, signatures, sig);
    let func_to_call_complexity = analytics.get(&sig.name).unwrap().values.clone();
    let per_arg_complexity = per_arg_complexity(signatures, sig, func_to_call_complexity);
    gen_analytics_values_for_func_call(f, sig, &per_arg_complexity)
}

fn gen_analytics_values_for_func_call(
    f: &Function,
    sig: &FunctionCallSignature,
    per_arg_complexity: &[ComplexityAnalyticsValue],
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    let mut values = HashMap::new();
    f.args.iter().for_each(|arg| {
        sig.args.iter().enumerate().for_each(|(p, e)| {
            if expr_contains_arg(e, arg) {
                merge_analytics_values(&mut values, arg, &per_arg_complexity[p]);
            }
        });
    });
    values
}

fn per_arg_complexity(
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
    sig: &FunctionCallSignature,
    func_to_call_complexity: HashMap<Ident, ComplexityAnalyticsValue>,
) -> Vec<ComplexityAnalyticsValue> {
    signatures
        .get(&sig.name)
        .unwrap()
        .args
        .iter()
        .map(|a| func_to_call_complexity.get(a).unwrap().clone())
        .collect()
}

fn gen_for_sub_func(
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
    sig: &FunctionCallSignature,
) {
    if analytics.get(&sig.name).is_none() {
        generate_complexity_analytics_concrete(
            &source_code_funcs.get(&sig.name).unwrap(),
            analytics,
            source_code_funcs,
            signatures,
        )
    }
}

fn expr_contains_arg(expr: &Expression, arg: &Ident) -> bool {
    match expr {
        Expression::FunctionArgument(name) if name == arg => true,
        Expression::SubExpression(exprs) => {
            exprs.iter().fold(
                false,
                |acc, e| {
                    if expr_contains_arg(e, arg) {
                        true
                    } else {
                        acc
                    }
                },
            )
        }
        _ => false,
    }
}

fn merge_analytics_values(
    values: &mut HashMap<Ident, ComplexityAnalyticsValue>,
    name: &Ident,
    value: &ComplexityAnalyticsValue,
) {
    let merged_value = match values.get(name) {
        None => value.clone(),
        Some(curr) => {
            if curr.cmp(&value) == Ordering::Less {
                value.clone()
            } else {
                curr.clone()
            }
        }
    };
    values.insert(name.clone(), merged_value);
}

fn merge_fn_analytics(
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<ComplexityAnalytics>>,
    initial_values: HashMap<Ident, ComplexityAnalyticsValue>,
) {
    let values = match analytics.get(&f.name.clone()) {
        Some(a) => complexity_analytics_values_per_ident(&initial_values, a),
        None => initial_values,
    };
    analytics.insert(f.name.clone(), Rc::new(ComplexityAnalytics { values }));
}

fn complexity_analytics_values_per_ident(
    values: &HashMap<Ident, ComplexityAnalyticsValue>,
    a: &Rc<ComplexityAnalytics>,
) -> HashMap<Ident, ComplexityAnalyticsValue> {
    a.values
        .iter()
        .map(|(n, av)| (n.clone(), av.clone()))
        .map(|(n, cav)| match values.get(&n) {
            Some(v) => {
                if cav.cmp(&v) == Ordering::Less {
                    (n, v.clone())
                } else {
                    (n, cav)
                }
            }
            None => (n, cav),
        })
        .collect()
}

fn analytics_parser(tokenizer: &mut Tokenizer) -> Result<ComplexityAnalytics, Errors> {
    let values = while_not_done_or_eof(
        HashMap::new(),
        |acc: HashMap<Ident, ComplexityAnalyticsValue>| {
            let arg = arg(tokenizer)?;
            shift_err(tokenizer.curr(), &mut |tok| match tok {
                PosTok {
                    tok: Tok::NL(_), ..
                } => done(tokenizer, &acc),
                PosTok {
                    tok: Tok::Ident(ident),
                    loc,
                } => complexity_value(tokenizer, &acc, &arg, &ident, loc),
                PosTok { loc, .. } => expected_nl_or_ident(&tokenizer, loc),
            })
        },
    )?;
    Ok(ComplexityAnalytics { values })
}

fn arg(tokenizer: &mut Tokenizer) -> Result<Ident, Errors> {
    spaces(tokenizer)?;
    let arg = read_ident(tokenizer)?;
    spaces(tokenizer)?;
    consume(&Tok::Ident("is".to_owned()), tokenizer)?;
    spaces(tokenizer)?;
    consume(&Tok::Ident("O".to_owned()), tokenizer)?;
    spaces(tokenizer)?;
    consume(&Tok::OpnBkt, tokenizer)?;
    spaces(tokenizer)?;
    Ok(arg)
}

fn expected_nl_or_ident(
    tokenizer: &&mut Tokenizer,
    loc: Loc,
) -> Result<State<HashMap<Ident, ComplexityAnalyticsValue>>, Vec<Error>> {
    Err(vec![Error {
        message: expected_msg(
            &[
                Tok::Ident("c".to_owned()),
                Tok::Ident("n".to_owned()),
                Tok::NL(1),
            ],
            tokenizer,
        ),
        loc,
    }])
}

fn complexity_value(
    tokenizer: &mut Tokenizer,
    acc: &HashMap<Ident, ComplexityAnalyticsValue>,
    arg: &Ident,
    ident: &str,
    loc: Loc,
) -> Result<State<HashMap<Ident, ComplexityAnalyticsValue>>, Vec<Error>> {
    let res = match ident {
        "c" => Some(ComplexityAnalyticsValue::OC),
        "n" => Some(ComplexityAnalyticsValue::ON),
        _ => None,
    };
    match res {
        Some(c) => {
            tokenizer.next();
            let mut hacc = acc.clone();
            hacc.insert(arg.clone(), c);
            go_on_if_comma(tokenizer, hacc)
        }
        None => Err(vec![Error {
            message: expected_msg(
                &[Tok::Ident("c".to_owned()), Tok::Ident("n".to_owned())],
                tokenizer,
            ),
            loc,
        }]),
    }
}

fn go_on_if_comma(
    mut tokenizer: &mut Tokenizer,
    hacc: HashMap<Ident, ComplexityAnalyticsValue>,
) -> Result<State<HashMap<Ident, ComplexityAnalyticsValue>>, Errors> {
    spaces(&mut tokenizer)?;
    consume(&Tok::ClsBkt, tokenizer)?;
    shift_err(tokenizer.curr(), &mut |tok| match tok {
        PosTok {
            tok: Tok::Comma, ..
        } => {
            tokenizer.next();
            Ok(State::GoOn(hacc.clone()))
        }
        PosTok {
            tok: Tok::NL(_), ..
        } => done(tokenizer, &hacc),
        PosTok { loc, .. } => Err(vec![Error {
            message: expected_msg(&[Tok::Comma, Tok::NL(1)], tokenizer),
            loc,
        }]),
    })
}

#[cfg(test)]
mod analytics {
    use aspects::complexity::analytics::generate_complexity_analytics;
    use aspects::complexity::model::{ComplexityAnalytics, ComplexityAnalyticsValue};
    use aspects::register::AnalyticsWrapper;
    use parsing::model::Ident;
    use semantic::model::{
        Expression, FnWithAnalytics, FuncName, Function, FunctionCallSignature, FunctionSignature,
    };
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    pub fn for_arg() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("f"),
            args: vec![Ident::new("arg")],
            body: Expression::FunctionArgument(Ident::new("arg")),
        };
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("f"), &function);
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics.get(&FuncName::new("f")).unwrap().analytics.len(),
            1
        );
        match analytics
            .get(&FuncName::new("f"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg")).unwrap(),
                    &ComplexityAnalyticsValue::OC
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn for_free_constant() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("f"),
            args: vec![Ident::new("arg")],
            body: Expression::Constant(Ident::new("some")),
        };
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("f"), &function);
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics.get(&FuncName::new("f")).unwrap().analytics.len(),
            1
        );
        match analytics
            .get(&FuncName::new("f"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg")).unwrap(),
                    &ComplexityAnalyticsValue::OC
                );
                assert_eq!(ca.values.get(&Ident::new("some")).is_none(), true);
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn for_external_function_call() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("f"),
            args: vec![Ident::new("arg1"), Ident::new("arg2")],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func"),
                args: vec![
                    Expression::FunctionArgument(Ident::new("arg2")),
                    Expression::FunctionArgument(Ident::new("arg1")),
                ],
            }),
        };
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("f"), &function);
        analytics.insert(
            FuncName::new("sub_func"),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![Ident::new("arg1"), Ident::new("arg2")],
                }),
                analytics: vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
                    values: {
                        let mut values = HashMap::new();
                        values.insert(Ident::new("arg1"), ComplexityAnalyticsValue::ON);
                        values.insert(Ident::new("arg2"), ComplexityAnalyticsValue::OC);
                        values
                    },
                }))],
            },
        );
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics.get(&FuncName::new("f")).unwrap().analytics.len(),
            1
        );
        match analytics
            .get(&FuncName::new("f"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg1")).unwrap(),
                    &ComplexityAnalyticsValue::OC
                );
                assert_eq!(
                    ca.values.get(&Ident::new("arg2")).unwrap(),
                    &ComplexityAnalyticsValue::ON
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn for_internal_function_call() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("func"),
            args: vec![Ident::new("arg1"), Ident::new("arg2")],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func"),
                args: vec![
                    Expression::FunctionArgument(Ident::new("arg1")),
                    Expression::FunctionArgument(Ident::new("arg2")),
                ],
            }),
        };
        let sub_function = Function {
            name: FuncName::new("sub_func"),
            args: vec![Ident::new("arg1"), Ident::new("arg2")],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_sub_func"),
                args: vec![
                    Expression::FunctionArgument(Ident::new("arg2")),
                    Expression::FunctionArgument(Ident::new("arg1")),
                ],
            }),
        };
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("func"), &function);
        source_code_funcs.insert(FuncName::new("sub_func"), &sub_function);
        analytics.insert(
            FuncName::new("sub_func"),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![Ident::new("arg1"), Ident::new("arg2")],
                }),
                analytics: vec![],
            },
        );
        analytics.insert(
            FuncName::new("sub_sub_func"),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: FuncName::new("sub_sub_func"),
                    args: vec![Ident::new("arg1"), Ident::new("arg2")],
                }),
                analytics: vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
                    values: {
                        let mut values = HashMap::new();
                        values.insert(Ident::new("arg1"), ComplexityAnalyticsValue::ON);
                        values.insert(Ident::new("arg2"), ComplexityAnalyticsValue::OC);
                        values
                    },
                }))],
            },
        );
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics
                .get(&FuncName::new("func"))
                .unwrap()
                .analytics
                .len(),
            1
        );
        match analytics
            .get(&FuncName::new("func"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg1")).unwrap(),
                    &ComplexityAnalyticsValue::OC
                );
                assert_eq!(
                    ca.values.get(&Ident::new("arg2")).unwrap(),
                    &ComplexityAnalyticsValue::ON
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn for_different_sub_expr() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("f"),
            args: vec![Ident::new("arg1")],
            body: Expression::SubExpression(vec![
                Expression::Constant(Ident::new("arg1")),
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![Expression::FunctionArgument(Ident::new("arg1"))],
                }),
            ]),
        };
        analytics.insert(
            FuncName::new("sub_func"),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![Ident::new("arg1")],
                }),
                analytics: vec![AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics {
                    values: {
                        let mut values = HashMap::new();
                        values.insert(Ident::new("arg1"), ComplexityAnalyticsValue::ON);
                        values
                    },
                }))],
            },
        );
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("f"), &function);
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics.get(&FuncName::new("f")).unwrap().analytics.len(),
            1
        );
        match analytics
            .get(&FuncName::new("f"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg1")).unwrap(),
                    &ComplexityAnalyticsValue::ON
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn for_equal_sub_expr() {
        let mut analytics = HashMap::new();
        let function = Function {
            name: FuncName::new("f"),
            args: vec![Ident::new("arg1")],
            body: Expression::SubExpression(vec![
                Expression::Constant(Ident::new("arg1")),
                Expression::Constant(Ident::new("arg1")),
            ]),
        };
        let mut source_code_funcs = HashMap::new();
        source_code_funcs.insert(FuncName::new("f"), &function);
        generate_complexity_analytics(&function, &mut analytics, &source_code_funcs);

        assert_eq!(
            analytics.get(&FuncName::new("f")).unwrap().analytics.len(),
            1
        );
        match analytics
            .get(&FuncName::new("f"))
            .unwrap()
            .analytics
            .get(0)
            .unwrap()
        {
            AnalyticsWrapper::Complexity(ca) => {
                assert_eq!(
                    ca.values.get(&Ident::new("arg1")).unwrap(),
                    &ComplexityAnalyticsValue::OC
                );
            }
            _ => panic!(),
        }
    }
}
