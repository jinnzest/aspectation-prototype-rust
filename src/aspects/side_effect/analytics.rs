use aspects::analytics::*;
use aspects::complexity::model::{ComplexityAnalytics, ComplexityAnalyticsValue};
use aspects::register::{AnalyticsWrapper, HintWrapper};
use aspects::side_effect::model::*;
use aspects::side_effect::parsing::{comma, console_input_or_output};
use parsing::model::{Error, Errors, Ident, Loc};
use parsing::tokenizer::{PosTok, Tok, Tokenizer};
use parsing::utils::{consume, done, expected_msg};
use parsing::utils::{shift_err, spaces, while_not_done_or_eof};
use semantic::model::FnWithAnalytics;
use semantic::model::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::rc::Rc;

pub fn generate_side_effect_analytics(
    func: &Function,
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    source_code_funcs: &HashMap<FuncName, &Function>,
) {
    let mut concrete_analytics =
        extract_concrete_analytics(analytics, &filter_side_effect_analytics);
    generate_side_effect_analytics_concrete(func, &mut concrete_analytics, source_code_funcs);
    merge_analytics(
        analytics,
        source_code_funcs,
        &mut concrete_analytics,
        &wrapper,
        &matcher_wrapper,
    );
}

pub fn filter_side_effect_analytics(aw: &AnalyticsWrapper) -> Option<Rc<SideEffectAnalytics>> {
    match aw {
        AnalyticsWrapper::SideEffect(a) => Some(a.clone()),
        _ => None,
    }
}

pub fn filter_side_effect_hint(aw: &HintWrapper) -> Option<Rc<SideEffectHint>> {
    match aw {
        HintWrapper::SideEffect(a) => Some(a.clone()),
        _ => None,
    }
}

fn wrapper(a: &Rc<SideEffectAnalytics>) -> AnalyticsWrapper {
    AnalyticsWrapper::SideEffect(a.clone())
}

fn matcher_wrapper(aw: &AnalyticsWrapper, a: Rc<SideEffectAnalytics>) -> AnalyticsWrapper {
    match aw {
        AnalyticsWrapper::SideEffect(_) => AnalyticsWrapper::SideEffect(a),
        other => other.clone(),
    }
}

fn generate_side_effect_analytics_concrete(
    f: &Function,
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
) -> HashSet<SideEffectAnalyticsValue> {
    let mut analytics_values = HashSet::new();
    analytics_values.insert(SideEffectAnalyticsValue::None);
    generate_analytics_for_body(analytics, source_code_funcs, &mut analytics_values, &f.body);
    cleanup_analytics(&mut analytics_values);
    analytics.insert(f.name.clone(), create_analytics(&mut analytics_values));
    analytics_values
}

fn generate_analytics_for_body(
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    mut analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
    expression: &Expression,
) {
    analytics_for_item(
        analytics,
        source_code_funcs,
        &mut analytics_values,
        expression,
    );
}

fn analytics_for_item(
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
    expr: &Expression,
) {
    use semantic::model::Expression::*;
    match expr {
        FunctionCall(sig) => {
            generate_analytics_for_identifier(
                analytics,
                source_code_funcs,
                analytics_values,
                &sig.name,
            );
            sig.args.iter().for_each(|e| {
                analytics_for_item(analytics, source_code_funcs, analytics_values, e)
            });
        }
        SubExpression(exprs) => {
            exprs.iter().for_each(|e| {
                analytics_for_item(analytics, source_code_funcs, analytics_values, e)
            });
        }
        _ => {
            analytics_values.insert(SideEffectAnalyticsValue::None);
        }
    };
}

fn generate_analytics_for_identifier(
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
    name: &FuncName,
) {
    if analytics.contains_key(name) {
        copy_from_analytics(analytics, analytics_values, name);
    } else if source_code_funcs.contains_key(&name) {
        generate_for_source_code_func(analytics, source_code_funcs, analytics_values, name)
    } else {
        panic!("unknown function {}", name);
    }
}

fn generate_for_source_code_func(
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
    n: &FuncName,
) {
    let func = source_code_funcs.get(&n).unwrap();
    let value = generate_side_effect_analytics_concrete(&func, analytics, source_code_funcs);
    analytics_values.extend(value);
}

fn copy_from_analytics(
    analytics: &mut HashMap<FuncName, Rc<SideEffectAnalytics>>,
    analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
    n: &FuncName,
) {
    analytics.get(&n).unwrap().values.iter().for_each(|v| {
        analytics_values.insert(v.clone());
    });
}

fn create_analytics(
    analytics_values: &mut HashSet<SideEffectAnalyticsValue>,
) -> Rc<SideEffectAnalytics> {
    Rc::new(SideEffectAnalytics {
        values: analytics_values.clone(),
    })
}

fn cleanup_analytics(func_analytics: &mut HashSet<SideEffectAnalyticsValue>) {
    if func_analytics.len() > 1 {
        func_analytics.remove(&SideEffectAnalyticsValue::None);
    }
}

pub fn create_side_effect_analytics(analytics: SideEffectAnalyticsValue) -> AnalyticsWrapper {
    let mut values = HashSet::new();
    values.insert(analytics);
    AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics { values }))
}

pub fn create_complexity_analytics(
    ident: &Ident,
    analytics: ComplexityAnalyticsValue,
) -> AnalyticsWrapper {
    let mut values = HashMap::new();
    values.insert(ident.clone(), analytics);
    AnalyticsWrapper::Complexity(Rc::new(ComplexityAnalytics { values }))
}

pub fn read_side_effect_analytics() -> Result<HashMap<FuncName, SideEffectAnalytics>, Errors> {
    read_analytics_from_file(&SideEffectAspect::name(), &analytics_parser)
}

fn analytics_parser(mut tokenizer: &mut Tokenizer) -> Result<SideEffectAnalytics, Errors> {
    let analytics = analytics(&mut tokenizer)?;
    Ok(analytics)
}

fn analytics(tokenizer: &mut Tokenizer) -> Result<SideEffectAnalytics, Vec<Error>> {
    shift_err(tokenizer.curr(), &mut |tok| match tok {
        PosTok {
            tok: Tok::Ident(ident),
            loc,
        } => match ident.as_ref() {
            "no" => no_side_effects_analytics(tokenizer),
            "allowed" => not_none_analytics(tokenizer),
            unexpected => expected_err(loc, &unexpected.to_owned()),
        },
        PosTok { tok, loc } => expected_err(loc, &tok),
    })
}

fn expected_err<R: Display>(loc: Loc, unexpected: &R) -> Result<SideEffectAnalytics, Vec<Error>> {
    Err(vec![Error {
        message: format!("Expected: 'none' or 'allowed'\nGot: '{}'", unexpected),
        loc,
    }])
}

fn no_side_effects_analytics(tokenizer: &mut Tokenizer) -> Result<SideEffectAnalytics, Vec<Error>> {
    tokenizer.next();
    consume_side_effects(tokenizer)?;
    let mut values = HashSet::new();
    values.insert(SideEffectAnalyticsValue::None);
    Ok(SideEffectAnalytics { values })
}

fn consume_side_effects(tokenizer: &mut Tokenizer) -> Result<(), Vec<Error>> {
    spaces(tokenizer)?;
    consume(&Tok::Ident("side".to_owned()), tokenizer)?;
    spaces(tokenizer)?;
    consume(&Tok::Ident("effects".to_owned()), tokenizer)
}

pub fn not_none_analytics(tokenizer: &mut Tokenizer) -> Result<SideEffectAnalytics, Vec<Error>> {
    tokenizer.next();
    consume_side_effects(tokenizer)?;
    spaces(tokenizer)?;
    consume(&Tok::Colon, tokenizer)?;
    let values: HashSet<SideEffectAnalyticsValue> =
        while_not_done_or_eof(HashSet::new(), &mut |acc: HashSet<
            SideEffectAnalyticsValue,
        >| {
            spaces(tokenizer)?;
            shift_err(tokenizer.curr(), |tok| match tok {
                PosTok {
                    tok: Tok::Comma, ..
                } => comma(tokenizer, &acc),
                PosTok {
                    tok: Tok::NL(_), ..
                } => done(tokenizer, &acc),
                PosTok {
                    tok: Tok::Ident(ident),
                    ..
                } if ident == "console" => console_input_or_output(tokenizer, &acc),
                PosTok { loc, .. } => Err(vec![Error {
                    message: expected_msg(
                        &[Tok::Comma, Tok::NL(0), Tok::Ident("console".to_owned())],
                        tokenizer,
                    ),
                    loc,
                }]),
            })
        })?;
    Ok(SideEffectAnalytics { values })
}

#[cfg(test)]
mod side_effect_analytics {
    use aspects::register::AnalyticsWrapper;
    use aspects::side_effect::analytics::generate_side_effect_analytics;
    use aspects::side_effect::model::{SideEffectAnalytics, SideEffectAnalyticsValue};
    use semantic::model::*;
    use std::collections::{HashMap, HashSet};
    use std::rc::Rc;

    #[test]
    pub fn no_side_effects_for_func_with_empty_body() {
        let name = FuncName::new("main_func");
        let f: Function = Function {
            name: name.clone(),
            args: vec![],
            body: Expression::SubExpression(vec![]),
        };
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();
        analytics.insert(
            name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: name.clone(),
                    args: vec![],
                }),
                analytics: vec![],
            },
        );
        let source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();

        generate_side_effect_analytics(&f, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics.get(&name).unwrap().analytics.first().unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 1);
                assert!(se.values.contains(&SideEffectAnalyticsValue::None));
            }
            _ => assert!(false, "should be only one analytics value"),
        };
    }

    #[test]
    pub fn side_effects_of_sub_func_to_func() {
        let main_name = FuncName::new("main_func");
        let main_func: Function = Function {
            name: main_name.clone(),
            args: vec![],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func"),
                args: vec![],
            }),
        };
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();
        let mut effects_sub_func = HashSet::new();
        effects_sub_func.insert(SideEffectAnalyticsValue::ConsoleInput);
        let sub_func_name = FuncName::new("sub_func");
        analytics.insert(
            sub_func_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func_name,
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func,
                }))],
            },
        );
        let mut source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();
        source_code_funcs.insert(main_name.clone(), &main_func);

        generate_side_effect_analytics(&main_func, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&main_name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics
            .get(&main_name)
            .unwrap()
            .analytics
            .first()
            .unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 1);
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleInput));
            }
            _ => assert!(false, "should be only one analytics kind"),
        };
    }

    #[test]
    pub fn side_effects_of_sub_sub_func_to_func() {
        let main_func_name = FuncName::new("main_func");
        let main_func: Function = Function {
            name: main_func_name.clone(),
            args: vec![],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func"),
                args: vec![],
            }),
        };
        let sub_func_name = FuncName::new("sub_func");
        let sub_func: Function = Function {
            name: sub_func_name.clone(),
            args: vec![],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func2"),
                args: vec![],
            }),
        };
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();

        let mut effects_sub_func = HashSet::new();
        effects_sub_func.insert(SideEffectAnalyticsValue::ConsoleInput);
        let sub_func2_name = FuncName::new("sub_func2");
        analytics.insert(
            sub_func2_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func2_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func,
                }))],
            },
        );
        let mut source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();
        source_code_funcs.insert(main_func_name.clone(), &main_func);
        source_code_funcs.insert(sub_func_name, &sub_func);

        generate_side_effect_analytics(&main_func, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&main_func_name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics
            .get(&main_func_name)
            .unwrap()
            .analytics
            .first()
            .unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 1);
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleInput));
            }
            _ => assert!(false, "should be only one analytics kind"),
        };
    }

    #[test]
    pub fn merge_side_effects_of_sub1_and_sub2_func_to_func() {
        let main_func_name = FuncName::new("main_func");
        let main_func: Function = Function {
            name: main_func_name.clone(),
            args: vec![],
            body: Expression::SubExpression(vec![
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![],
                }),
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func2"),
                    args: vec![],
                }),
            ]),
        };
        let sub_func_name = FuncName::new("sub_func");
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();

        let mut effects_sub_func1 = HashSet::new();
        effects_sub_func1.insert(SideEffectAnalyticsValue::ConsoleInput);
        let mut effects_sub_func2 = HashSet::new();
        effects_sub_func2.insert(SideEffectAnalyticsValue::ConsoleOutput);
        let sub_func2_name = FuncName::new("sub_func2");
        analytics.insert(
            sub_func_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func1,
                }))],
            },
        );
        analytics.insert(
            sub_func2_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func2_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func2,
                }))],
            },
        );
        let mut source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();
        source_code_funcs.insert(main_func_name.clone(), &main_func);

        generate_side_effect_analytics(&main_func, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&main_func_name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics
            .get(&main_func_name)
            .unwrap()
            .analytics
            .first()
            .unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 2);
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleInput));
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleOutput));
            }
            _ => assert!(false, "should be only one analytics kind"),
        };
    }

    #[test]
    pub fn merge_side_effects_of_sub1_intern_and_sub2_extern_func_to_func() {
        let main_func_name = FuncName::new("main_func");
        let main_func: Function = Function {
            name: main_func_name.clone(),
            args: vec![],
            body: Expression::SubExpression(vec![
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![],
                }),
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func2"),
                    args: vec![],
                }),
            ]),
        };
        let sub_func2_name = FuncName::new("sub_func2");
        let sub_func2: Function = Function {
            name: sub_func2_name.clone(),
            args: vec![],
            body: Expression::FunctionCall(FunctionCallSignature {
                name: FuncName::new("sub_func3"),
                args: vec![],
            }),
        };
        let sub_func_name3 = FuncName::new("sub_func3");
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();
        let mut effects_sub_func3 = HashSet::new();
        effects_sub_func3.insert(SideEffectAnalyticsValue::ConsoleInput);
        analytics.insert(
            sub_func_name3.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func_name3.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func3,
                }))],
            },
        );

        let mut effects_sub_func = HashSet::new();
        effects_sub_func.insert(SideEffectAnalyticsValue::ConsoleOutput);
        let sub_func_name = FuncName::new("sub_func");
        analytics.insert(
            sub_func_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func,
                }))],
            },
        );
        let mut source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();
        source_code_funcs.insert(main_func_name.clone(), &main_func);
        source_code_funcs.insert(sub_func2_name.clone(), &sub_func2);

        generate_side_effect_analytics(&main_func, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&main_func_name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics
            .get(&main_func_name)
            .unwrap()
            .analytics
            .first()
            .unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 2);
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleInput));
                assert!(se.values.contains(&SideEffectAnalyticsValue::ConsoleOutput));
            }
            _ => assert!(false, "should be only one analytics kind"),
        };
    }

    #[test]
    pub fn merge_none_side_effects_of_sub1_and_sub2_func_to_func() {
        let main_func_name = FuncName::new("main_func");
        let main_func: Function = Function {
            name: main_func_name.clone(),
            args: vec![],
            body: Expression::SubExpression(vec![
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func"),
                    args: vec![],
                }),
                Expression::FunctionCall(FunctionCallSignature {
                    name: FuncName::new("sub_func2"),
                    args: vec![],
                }),
            ]),
        };
        let sub_func_name = FuncName::new("sub_func");
        let mut analytics: HashMap<FuncName, FnWithAnalytics> = HashMap::new();

        let mut effects_sub_func1 = HashSet::new();
        effects_sub_func1.insert(SideEffectAnalyticsValue::None);
        let mut effects_sub_func2 = HashSet::new();
        effects_sub_func2.insert(SideEffectAnalyticsValue::None);
        let sub_func2_name = FuncName::new("sub_func2");
        analytics.insert(
            sub_func_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func1,
                }))],
            },
        );
        analytics.insert(
            sub_func2_name.clone(),
            FnWithAnalytics {
                sig: Rc::new(FunctionSignature {
                    name: sub_func2_name.clone(),
                    args: vec![],
                }),
                analytics: vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: effects_sub_func2,
                }))],
            },
        );
        let mut source_code_funcs: HashMap<FuncName, &Function> = HashMap::new();
        source_code_funcs.insert(main_func_name.clone(), &main_func);

        generate_side_effect_analytics(&main_func, &mut analytics, &source_code_funcs);

        assert_eq!(analytics.get(&main_func_name).unwrap().analytics.len(), 1);
        let analytics_wrapper = analytics
            .get(&main_func_name)
            .unwrap()
            .analytics
            .first()
            .unwrap();
        match analytics_wrapper {
            AnalyticsWrapper::SideEffect(se) => {
                assert_eq!(se.values.len(), 1);
                assert!(se.values.contains(&SideEffectAnalyticsValue::None));
            }
            _ => assert!(false, "should be only one analytics kind"),
        };
    }
}
