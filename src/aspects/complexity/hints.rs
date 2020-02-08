use aspects::complexity::model::ComplexityAspect;
use aspects::complexity::model::*;
use aspects::hints::{read_hints, write_hints};
use aspects::model::HintFields;
use aspects::register::HintWrapper;
use parsing::model::{Error, Errors, Ident, Loc};
use parsing::tokenizer::{ident, PosTok, Tok, Tokenizer};
use parsing::utils::{
    consume, done, expected_msg, go_on, shift_err, spaces, while_not_done_or_eof, State,
};
use semantic::model::*;
use std::collections::HashMap;
use std::rc::Rc;

pub fn read_complexity_hints() -> Result<HashMap<FuncName, ComplexityHint>, Errors> {
    read_hints(&ComplexityAspect::name(), &hint_parser)
}

pub fn write_complexity_hints(hint_fields: &[(FuncName, HintFields)]) {
    write_hints(&ComplexityAspect::name(), hint_fields, &|hw| match hw {
        HintWrapper::Complexity(_) => Some(Rc::new(hw.clone())),
        _ => None,
    });
}

fn hint_parser(tokenizer: &mut Tokenizer) -> Result<ComplexityHint, Errors> {
    let hint = hint(tokenizer)?;
    Ok(hint)
}

fn hint(tokenizer: &mut Tokenizer) -> Result<ComplexityHint, Vec<Error>> {
    let values = while_not_done_or_eof(
        HashMap::new(),
        |hacc: HashMap<Ident, ComplexityHintValue>| {
            spaces(tokenizer)?;
            shift_err(tokenizer.curr(), &mut |tok| match tok {
                PosTok {
                    tok: Tok::NL(_), ..
                } => done(tokenizer, &hacc),
                PosTok {
                    tok: Tok::Ident(ident),
                    loc,
                } => complexity_per_arg(tokenizer, ident, &hacc, loc),
                PosTok {
                    tok: Tok::Comma, ..
                } => go_on(tokenizer, &hacc),
                PosTok { loc, .. } => hint_error(&tokenizer, loc),
            })
        },
    )?;
    Ok(ComplexityHint { values })
}

fn hint_error(
    tokenizer: &&mut Tokenizer,
    loc: Loc,
) -> Result<State<HashMap<Ident, ComplexityHintValue>>, Vec<Error>> {
    Err(vec![Error {
        message: expected_msg(&[Tok::NL(0), Tok::Comma, ident("")], tokenizer),
        loc,
    }])
}

fn complexity_per_arg(
    tokenizer: &mut Tokenizer,
    arg: String,
    hacc: &HashMap<Ident, ComplexityHintValue>,
    loc: Loc,
) -> Result<State<HashMap<Ident, ComplexityHintValue>>, Vec<Error>> {
    tokenizer.next();
    spaces(tokenizer)?;
    consume(&Tok::Colon, tokenizer)?;
    spaces(tokenizer)?;
    let res = complexity_value(tokenizer)?;
    match res {
        Some(c) => {
            tokenizer.next();
            let mut hacc = hacc.clone();
            hacc.insert(Ident::new(&arg), c);
            Ok(State::GoOn(hacc))
        }
        None => expected_err_msg(tokenizer, loc),
    }
}

fn complexity_value(tokenizer: &mut Tokenizer) -> Result<Option<ComplexityHintValue>, Vec<Error>> {
    shift_err(tokenizer.curr(), &mut |tok| match tok {
        PosTok {
            tok: Tok::Ident(ident),
            ..
        } => Ok(match ident.as_str() {
            "c" => Some(ComplexityHintValue::OC),
            "n" => Some(ComplexityHintValue::ON),
            "any" => Some(ComplexityHintValue::Any),
            _ => None,
        }),
        PosTok { loc, .. } => Err(vec![Error {
            message: expected_msg(&[ident("c"), ident("n"), ident("any")], tokenizer),
            loc,
        }]),
    })
}

fn expected_err_msg(
    tokenizer: &mut Tokenizer,
    loc: Loc,
) -> Result<State<HashMap<Ident, ComplexityHintValue>>, Vec<Error>> {
    Err(vec![Error {
        message: expected_msg(
            &[
                Tok::Ident("c".to_owned()),
                Tok::Ident("n".to_owned()),
                Tok::Ident("any".to_owned()),
            ],
            tokenizer,
        ),
        loc,
    }])
}
