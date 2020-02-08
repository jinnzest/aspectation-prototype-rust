use aspects::side_effect::model::SideEffectAnalyticsValue;
use parsing::model::{Error, Loc};
use parsing::tokenizer::{PosTok, Tok, Tokenizer};
use parsing::utils::{expected_msg, shift_err, spaces, State};
use std::collections::HashSet;

pub fn console_input_or_output(
    tokenizer: &mut Tokenizer,
    acc: &HashSet<SideEffectAnalyticsValue>,
) -> Result<State<HashSet<SideEffectAnalyticsValue>>, Vec<Error>> {
    tokenizer.next();
    spaces(tokenizer)?;
    shift_err(tokenizer.curr(), |tok| match tok {
        PosTok {
            tok: Tok::Ident(ident),
            loc,
        } => {
            if &ident == "input" {
                console_input_value(tokenizer, &acc)
            } else if &ident == "output" {
                console_output_values(tokenizer, &acc)
            } else {
                expected_console_input_or_output(tokenizer, loc)
            }
        }
        PosTok { loc, .. } => Err(vec![Error {
            message: expected_msg(
                &[
                    Tok::Ident("console input".to_owned()),
                    Tok::Ident("console output".to_owned()),
                ],
                tokenizer,
            ),
            loc,
        }]),
    })
}

fn expected_console_input_or_output(
    tokenizer: &mut Tokenizer,
    loc: Loc,
) -> Result<State<HashSet<SideEffectAnalyticsValue>>, Vec<Error>> {
    Err(vec![Error {
        message: expected_msg(
            &[
                Tok::Ident("input".to_owned()),
                Tok::Ident("output".to_owned()),
            ],
            tokenizer,
        ),
        loc,
    }])
}

fn console_input_value(
    tokenizer: &mut Tokenizer,
    acc: &HashSet<SideEffectAnalyticsValue>,
) -> Result<State<HashSet<SideEffectAnalyticsValue>>, Vec<Error>> {
    let mut cloned_acc = acc.clone();
    tokenizer.next();
    cloned_acc.insert(SideEffectAnalyticsValue::ConsoleInput);
    Ok(State::GoOn(cloned_acc))
}

fn console_output_values(
    tokenizer: &mut Tokenizer,
    acc: &HashSet<SideEffectAnalyticsValue>,
) -> Result<State<HashSet<SideEffectAnalyticsValue>>, Vec<Error>> {
    let mut cloned_acc = acc.clone();
    tokenizer.next();
    cloned_acc.insert(SideEffectAnalyticsValue::ConsoleOutput);
    Ok(State::GoOn(cloned_acc))
}

pub fn comma(
    tokenizer: &mut Tokenizer,
    acc: &HashSet<SideEffectAnalyticsValue>,
) -> Result<State<HashSet<SideEffectAnalyticsValue>>, Vec<Error>> {
    tokenizer.next();
    Ok(State::GoOn(acc.clone()))
}
