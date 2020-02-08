use aspects::model::HintFields;
use morphism_parser::parser;
use morphism_parser::parser::{Constr, Expr};
use parsing::model::Error;
use parsing::model::Errors;
use parsing::model::Loc;
use parsing::model::{Ident, Ranged};
use semantic::model::{
    Construction, Expression, FnWithAnalytics, FuncName, Function, FunctionCallSignature,
    FunctionSignature,
};
use semantic::utils::str_by_comma;
use std::collections::HashMap;
use std::rc::Rc;

pub fn mk_semantic_tree(
    constrs: &[Constr],
    hints: &HashMap<FuncName, HintFields>,
    external_functions: &HashMap<FuncName, FnWithAnalytics>,
) -> Result<Vec<Construction>, Errors> {
    let mut function_signatures: HashMap<FuncName, Rc<FunctionSignature>> = external_functions
        .iter()
        .map(|(n, fa)| (n.clone(), fa.sig.clone()))
        .collect();
    constrs.iter().for_each(|c| match c {
        Constr::Func(f) => {
            function_signatures.insert(
                FuncName::new_from_ident(&f.v.name.v),
                Rc::new(FunctionSignature {
                    name: FuncName::new_from_ident(&f.v.name.v),
                    args: f.v.args.iter().map(|a| a.v.clone()).collect(),
                }),
            );
        }
    });
    constrs
        .iter()
        .map(|c| match c {
            Constr::Func(f) => {
                let curr = f.v.body[0].v.clone();
                let mut itr = f.v.body.iter();
                let name = FuncName::new_from_ident(&f.v.name.v);
                let body = mk_tree_for_expr(&name, curr, &mut itr, hints, &function_signatures)?;
                Ok(Construction::Function(Function {
                    name,
                    args: f.v.args.iter().map(|a| a.v.clone()).collect(),
                    body,
                }))
            }
        })
        .collect()
}

fn mk_tree_for_expr<'e, I>(
    f: &FuncName,
    func_call: Expr,
    body_iter: &mut I,
    hints: &HashMap<FuncName, HintFields>,
    function_signatures: &HashMap<FuncName, Rc<FunctionSignature>>,
) -> Result<Expression, Errors>
where
    I: Iterator<Item = &'e Ranged<Expr>>,
{
    let eo = body_iter.next();
    match eo {
        Some(e) => match e {
            Ranged {
                v: Expr::Int(parser::Int { val: int, .. }),
                ..
            } => Ok(Expression::Constant(Ident::new(int))),
            Ranged {
                v: Expr::SubExpr(exprs),
                ..
            } => {
                let curr = exprs[0].v.clone();
                let mut itr = exprs.iter();
                mk_tree_for_expr(f, curr, &mut itr, hints, function_signatures)
            }
            Ranged {
                v: Expr::Ident(ident),
                ..
            } => {
                let name = FuncName::new(ident.str());
                let res = match function_signatures.get(&name) {
                    Some(signature) => {
                        let args: Vec<Expression> = signature
                            .args
                            .iter()
                            .map(|_| {
                                mk_tree_for_expr(
                                    &f,
                                    func_call.clone(),
                                    body_iter,
                                    hints,
                                    function_signatures,
                                )
                            })
                            .try_fold(vec![], |mut acc, r| match r {
                                Ok(v) => {
                                    acc.push(v);
                                    Ok(acc)
                                }
                                Err(err) => {
                                    let mut errors: Vec<Error> = vec![str_by_comma(&acc)]
                                        .iter()
                                        .map(|s| Error {
                                            message: s.clone(),
                                            loc: Loc {
                                                pos: 0,
                                                line: 0,
                                                col: 0,
                                            },
                                        })
                                        .collect();
                                    errors.extend(err);
                                    Err(errors)
                                }
                            })?;
                        Ok(Expression::FunctionCall(FunctionCallSignature {
                            name: name.clone(),
                            args,
                        }))
                    }
                    None => Err(vec![Error {
                        message: format!("no function with name {} found", name),
                        loc: Loc {
                            pos: 0,
                            line: 0,
                            col: 0,
                        },
                    }]),
                };
                match res {
                    Ok(_) => res,
                    Err(err) => {
                        let args: Vec<String> = function_signatures
                            .get(&f)
                            .unwrap()
                            .args
                            .iter()
                            .map(|i| i.str().to_owned())
                            .collect();
                        if args.contains(&name.str().to_owned()) {
                            Ok(Expression::FunctionArgument(Ident::new(name.str())))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
        },
        None => {
            let name = match func_call {
                Expr::Ident(ident) => ident.str().to_owned(),
                other => panic!("OTHER: {:?}", other),
            };
            Err(vec![Error {
                message: format!(
                    "Function call {:?} expected arguments: {:?}\nGot: ",
                    name,
                    function_signatures
                        .get(&FuncName::new(&name))
                        .unwrap()
                        .args
                        .iter()
                        .map(|i| i.str().to_owned())
                        .collect::<Vec<String>>()
                ),
                loc: Loc {
                    pos: 0,
                    line: 0,
                    col: 0,
                },
            }])
        }
    }
}
