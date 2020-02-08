use aspects::hints::{read_hints, write_hints};
use aspects::model::HintFields;
use aspects::register::HintWrapper;
use aspects::side_effect::model::*;
use aspects::side_effect::parsing::{comma, console_input_or_output};
use parsing::model::{Error, Errors};
use parsing::tokenizer::{PosTok, Tok, Tokenizer};
use parsing::utils::{done, shift_err, spaces, while_not_done_or_eof};
use semantic::model::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub fn read_side_effect_hints() -> Result<HashMap<FuncName, SideEffectHint>, Errors> {
    read_hints(&SideEffectAspect::name(), &hint_parser)
}

pub fn write_side_effect_hints(hint_fields: &[(FuncName, HintFields)]) {
    write_hints(&SideEffectAspect::name(), hint_fields, &|hw| match hw {
        HintWrapper::SideEffect(_) => Some(Rc::new(hw.clone())),
        _ => None,
    });
}

fn hint_parser(mut tokenizer: &mut Tokenizer) -> Result<SideEffectHint, Errors> {
    let hint = hint(&mut tokenizer)?;
    Ok(hint)
}

fn hint(tokenizer: &mut Tokenizer) -> Result<SideEffectHint, Vec<Error>> {
    shift_err(tokenizer.curr(), &mut |tok| match tok {
        PosTok {
            tok: Tok::Ident(ident),
            ..
        } => match ident.as_ref() {
            "any" => {
                tokenizer.next();
                Ok(SideEffectHint::AnySideEffect)
            }
            "none" => {
                tokenizer.next();
                Ok(SideEffectHint::NoSideEffects)
            }
            _ => allowed_effects_parser(tokenizer),
        },
        PosTok { tok, loc } => Err(vec![Error {
            message: format!("Expected one of 'any', 'no' or 'allowed'\nGot: '{}'", tok),
            loc,
        }]),
    })
}

fn allowed_effects_parser(mut tokenizer: &mut Tokenizer) -> Result<SideEffectHint, Errors> {
    let analytics = not_none_hints(&mut tokenizer)?;
    Ok(SideEffectHint::AllowedSideEffects(analytics))
}

fn not_none_hints(tokenizer: &mut Tokenizer) -> Result<SideEffectAnalytics, Vec<Error>> {
    let values: HashSet<SideEffectAnalyticsValue> =
        while_not_done_or_eof(HashSet::new(), |acc: HashSet<SideEffectAnalyticsValue>| {
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
                PosTok { tok, loc } => Err(vec![Error {
                    message: format!(
                        "Expected one of 'console input', 'console output' or 'any'\nGot: {}",
                        tok
                    ),
                    loc,
                }]),
            })
        })?;
    Ok(SideEffectAnalytics { values })
}
