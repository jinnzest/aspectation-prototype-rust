use parsing::lines_reader::LinesReader;
use parsing::model::{Error, Errors, Ident, Loc, Range, Ranged};
use parsing::tokenizer::Tokenizer;
use parsing::tokenizer::{PosTok, Tok};
use parsing::utils::{expected_msg, shift_err, spaces, while_not_done_or_eof, State};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Int {
    pub val: String,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Func {
    pub name: Ranged<Ident>,
    pub args: Vec<Ranged<Ident>>,
    pub body: Vec<Ranged<Expr>>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Expr {
    Ident(Ident),
    Int(Int),
    SubExpr(Vec<Ranged<Expr>>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Constr {
    Func(Ranged<Func>),
}

pub struct MorphismParser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> MorphismParser<'a> {
    pub fn new(reader: &'a mut LinesReader<'a>) -> MorphismParser {
        let tokenizer = Tokenizer::new(reader);
        MorphismParser { tokenizer }
    }
    pub fn line_pos(&self) -> Vec<usize> {
        self.tokenizer.line_pos()
    }
    pub fn go_to_eof(&mut self) {
        while self.tokenizer.curr().is_ok() && self.tokenizer.curr().unwrap().tok != Tok::EOF {
            self.tokenizer.next();
        }
    }
    pub fn parse(&mut self) -> Result<Vec<Constr>, Errors> {
        while_not_done_or_eof(vec![], |c: Vec<Constr>| {
            spaces(&mut self.tokenizer)?;
            shift_err(self.tokenizer.curr(), |tok| match tok {
                PosTok { tok: Tok::EOF, .. } => Ok(State::Done(c.clone())),
                PosTok {
                    tok: Tok::NL(_), ..
                } => {
                    self.tokenizer.next();
                    Ok(State::GoOn(c.clone()))
                }
                _ => match self.line() {
                    Ok(constr) => {
                        let mut consts = c.clone();
                        consts.push(constr);
                        Ok(State::GoOn(consts))
                    }
                    Err(errors) => Err(errors),
                },
            })
        })
    }
    fn line(&mut self) -> Result<Constr, Errors> {
        match self.constr() {
            Ok(constr) => Ok(constr),
            Err(err) => Err(err),
        }
    }
    fn constr(&mut self) -> Result<Constr, Errors> {
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok { tok: Tok::Fn, loc } => {
                self.tokenizer.next();
                self.func(loc)
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::Fn], &self.tokenizer),
                loc,
            }]),
        })
    }
    fn func(&mut self, begin: Loc) -> Result<Constr, Errors> {
        let name = self.name()?;
        let args = self.args()?;
        self.eq()?;
        let body = self.body()?;
        Ok(Constr::Func(Ranged {
            v: Func {
                name,
                args,
                body: body.clone(),
            },
            range: Range {
                begin,
                end: body[body.len() - 1].range.end,
            },
        }))
    }
    fn eq(&mut self) -> Result<Vec<Ranged<Expr>>, Errors> {
        spaces(&mut self.tokenizer)?;
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok { tok: Tok::Eq, .. } => {
                self.tokenizer.next();
                Ok(vec![])
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::Eq], &self.tokenizer),
                loc,
            }]),
        })
    }
    fn body(&mut self) -> Result<Vec<Ranged<Expr>>, Errors> {
        self.expr(0)
    }
    fn expr(&mut self, level: usize) -> Result<Vec<Ranged<Expr>>, Errors> {
        while_not_done_or_eof(vec![], |b: Vec<Ranged<Expr>>| {
            spaces(&mut self.tokenizer)?;
            shift_err(self.tokenizer.curr(), |tok| match tok {
                PosTok {
                    tok: Tok::Ident(val),
                    loc,
                } => {
                    self.tokenizer.next();
                    let curr = self.tokenizer.curr()?;
                    let mut body = b.clone();
                    body.push(Ranged {
                        v: Expr::Ident(Ident::new(&val)),
                        range: Range {
                            begin: loc,
                            end: curr.loc,
                        },
                    });
                    Ok(State::GoOn(body))
                }
                PosTok {
                    tok: Tok::Int(val),
                    loc,
                } => {
                    self.tokenizer.next();
                    let curr = self.tokenizer.curr()?;
                    let mut body = b.clone();
                    body.push(Ranged {
                        v: Expr::Int(Int { val }),
                        range: Range {
                            begin: loc,
                            end: curr.loc,
                        },
                    });
                    Ok(State::GoOn(body))
                }
                PosTok { tok: Tok::EOF, loc } => {
                    let mut body = b.clone();
                    expr_eof(level, &mut body, &self.tokenizer, loc)
                }
                PosTok {
                    tok: Tok::OpnBkt,
                    loc,
                } => {
                    self.tokenizer.next();
                    let sub_expr = self.expr(level + 1)?;
                    let curr = self.tokenizer.curr()?;
                    let mut body = b.clone();
                    body.push(Ranged {
                        v: Expr::SubExpr(sub_expr),
                        range: Range {
                            begin: loc,
                            end: curr.loc,
                        },
                    });
                    Ok(State::GoOn(body))
                }
                PosTok {
                    tok: Tok::ClsBkt, ..
                } => {
                    self.tokenizer.next();
                    Ok(State::Done(b.clone()))
                }
                PosTok {
                    tok: Tok::NL(_),
                    loc,
                } => {
                    if level == 0 {
                        Ok(State::Done(b.clone()))
                    } else {
                        Err(vec![Error {
                            message: expected_msg(&[Tok::Ident("".to_owned())], &self.tokenizer),
                            loc,
                        }])
                    }
                }
                PosTok { loc, .. } => Err(vec![Error {
                    message: expected_msg(
                        &[
                            Tok::Ident("".to_owned()),
                            Tok::Int("".to_owned()),
                            Tok::OpnBkt,
                            Tok::ClsBkt,
                            Tok::NL(0),
                        ],
                        &self.tokenizer,
                    ),
                    loc,
                }]),
            })
        })
    }
    fn name(&mut self) -> Result<Ranged<Ident>, Errors> {
        spaces(&mut self.tokenizer)?;
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok {
                tok: Tok::Ident(val),
                loc,
            } => {
                self.tokenizer.next();
                let curr = self.tokenizer.curr()?;
                Ok(Ranged {
                    v: Ident::new(&val),
                    range: Range {
                        begin: loc,
                        end: curr.loc,
                    },
                })
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::Ident("".to_owned())], &self.tokenizer),
                loc,
            }]),
        })
    }
    fn args(&mut self) -> Result<Vec<Ranged<Ident>>, Errors> {
        spaces(&mut self.tokenizer)?;
        while_not_done_or_eof(vec![], |a: Vec<Ranged<Ident>>| {
            shift_err(self.tokenizer.curr(), |tok| match tok {
                PosTok {
                    tok: Tok::Ident(val),
                    loc,
                } => {
                    self.tokenizer.next();
                    let curr = self.tokenizer.curr()?;
                    let mut args = a.clone();
                    args.push(Ranged {
                        v: Ident::new(&val),
                        range: Range {
                            begin: loc,
                            end: curr.loc,
                        },
                    });
                    spaces(&mut self.tokenizer)?;
                    Ok(State::GoOn(args))
                }
                PosTok { tok: Tok::Eq, .. } => Ok(State::Done(a.clone())),
                PosTok { loc, .. } => Err(vec![Error {
                    message: expected_msg(&[Tok::Ident("".to_owned()), Tok::Eq], &self.tokenizer),
                    loc,
                }]),
            })
        })
    }
}

fn expr_eof(
    level: usize,
    body: &mut Vec<Ranged<Expr>>,
    tokenizer: &Tokenizer,
    loc: Loc,
) -> Result<State<Vec<Ranged<Expr>>>, Vec<Error>> {
    if body.is_empty() {
        if level > 0 {
            Err(vec![Error {
                message: expected_msg(
                    &[Tok::Ident("".to_owned()), Tok::OpnBkt, Tok::ClsBkt],
                    tokenizer,
                ),
                loc,
            }])
        } else {
            Err(vec![Error {
                message: expected_msg(&[Tok::Ident("".to_owned()), Tok::OpnBkt], tokenizer),
                loc,
            }])
        }
    } else {
        Ok(State::Done(body.to_vec()))
    }
}

#[cfg(test)]
mod morphism_parser {
    use super::*;
    use parsing::model::Loc;

    #[test]
    fn eof() {
        let mut reader = LinesReader::new("");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap().len(), 0);
    }

    #[test]
    fn skip_new_lines_and_spaces() {
        let mut reader = LinesReader::new(" \n \n  \n   \n \n\n ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap().len(), 0);
    }

    #[test]
    fn space_only() {
        let mut reader = LinesReader::new(" ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap().len(), 0);
    }

    #[test]
    fn no_fn_name_and_body() {
        let mut reader = LinesReader::new(" fn ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Expected: identifier\nGot: \'EOF\' at 1:5"
        );
    }

    #[test]
    fn fn_no_eq_no_args() {
        let mut reader = LinesReader::new(" fn name");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Expected: identifier or '='\nGot: 'EOF' at 1:9"
        );
    }

    #[test]
    fn fn_args_but_no_eq() {
        let mut reader = LinesReader::new(" fn name arg1 arg2 ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Expected: identifier or '='\nGot: 'EOF' at 1:20"
        );
    }

    #[test]
    fn fn_no_body() {
        let mut reader = LinesReader::new(" fn name arg1 arg2 = ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "Expected: identifier or '('\nGot: 'EOF' at 1:22"
        );
        assert_eq!(
            errors[0].loc,
            Loc {
                pos: 21,
                line: 1,
                col: 22,
            }
        );
    }

    #[test]
    fn parse_fn() {
        let mut reader = LinesReader::new(" fn name arg1 arg2 = body1 body2");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn parse_fn_sub_expr() {
        let mut reader = LinesReader::new(" fn name arg1 arg2 = body1 (body2 1 ) ");
        let mut parser = MorphismParser::new(&mut reader);
        let result = parser.parse();
        assert!(result.is_ok());
        let name = Ranged {
            v: Ident::new("name"),
            range: Range {
                begin: Loc {
                    pos: 4,
                    line: 1,
                    col: 5,
                },
                end: Loc {
                    pos: 8,
                    line: 1,
                    col: 9,
                },
            },
        };
        let args = vec![
            Ranged {
                v: Ident::new("arg1"),
                range: Range {
                    begin: Loc {
                        pos: 9,
                        line: 1,
                        col: 10,
                    },
                    end: Loc {
                        pos: 13,
                        line: 1,
                        col: 14,
                    },
                },
            },
            Ranged {
                v: Ident::new("arg2"),
                range: Range {
                    begin: Loc {
                        pos: 14,
                        line: 1,
                        col: 15,
                    },
                    end: Loc {
                        pos: 18,
                        line: 1,
                        col: 19,
                    },
                },
            },
        ];
        let body = vec![
            Ranged {
                v: Expr::Ident(Ident::new("body1")),
                range: Range {
                    begin: Loc {
                        pos: 21,
                        line: 1,
                        col: 22,
                    },
                    end: Loc {
                        pos: 26,
                        line: 1,
                        col: 27,
                    },
                },
            },
            Ranged {
                v: Expr::SubExpr(vec![
                    Ranged {
                        v: Expr::Ident(Ident::new("body2")),
                        range: Range {
                            begin: Loc {
                                pos: 28,
                                line: 1,
                                col: 29,
                            },
                            end: Loc {
                                pos: 33,
                                line: 1,
                                col: 34,
                            },
                        },
                    },
                    Ranged {
                        v: Expr::Int(Int {
                            val: "1".to_owned(),
                        }),
                        range: Range {
                            begin: Loc {
                                pos: 34,
                                line: 1,
                                col: 35,
                            },
                            end: Loc {
                                pos: 35,
                                line: 1,
                                col: 36,
                            },
                        },
                    },
                ]),
                range: Range {
                    begin: Loc {
                        pos: 27,
                        line: 1,
                        col: 28,
                    },
                    end: Loc {
                        pos: 37,
                        line: 1,
                        col: 38,
                    },
                },
            },
        ];
        assert_eq!(
            result.ok().unwrap(),
            [Constr::Func(Ranged {
                v: Func { name, args, body },
                range: Range {
                    begin: Loc {
                        pos: 1,
                        line: 1,
                        col: 2,
                    },
                    end: Loc {
                        pos: 37,
                        line: 1,
                        col: 38,
                    },
                },
            })]
        )
    }
}
