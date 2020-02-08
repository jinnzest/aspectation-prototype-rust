use parsing::model::{Error, Errors, Ident};
use parsing::tokenizer::{PosTok, Tok, Tokenizer};

#[derive(Clone)]
pub enum State<R> {
    Done(R),
    GoOn(R),
}

pub fn while_not_done_or_eof<F, R>(init: R, mut f: F) -> Result<R, Errors>
where
    F: FnMut(R) -> Result<State<R>, Errors>,
{
    let mut state = Ok(State::GoOn(init));
    while not_done(&state) {
        if let Ok(State::GoOn(s)) = state {
            state = f(s)
        }
    }
    match state {
        Ok(State::Done(s)) => Ok(s),
        Err(err) => Err(err),
        _ => panic!("must be in done state after while is finished"),
    }
}

pub fn not_done<R>(state: &Result<State<R>, Errors>) -> bool {
    match state {
        Ok(State::Done(_)) => false,
        Err(_) => false,
        _ => true,
    }
}

pub fn shift_err<F, R>(curr: Result<PosTok, Errors>, mut f: F) -> Result<R, Errors>
where
    F: FnMut(PosTok) -> Result<R, Errors>,
{
    match curr {
        Ok(ok) => f(ok),
        Err(err) => Err(err),
    }
}

pub fn spaces(tokenizer: &mut Tokenizer) -> Result<(), Errors> {
    shift_err(tokenizer.curr(), |tok| match tok {
        PosTok {
            tok: Tok::Sps(_), ..
        } => {
            tokenizer.next();
            Ok(())
        }
        _ => Ok(()),
    })
}

pub fn consume(expected_tok: &Tok, tokenizer: &mut Tokenizer) -> Result<(), Errors> {
    shift_err(tokenizer.curr(), |tok| match tok {
        PosTok { tok, .. } if tok == *expected_tok => {
            tokenizer.next();
            Ok(())
        }
        PosTok { loc, .. } => Err(vec![Error {
            message: expected_msg(&[expected_tok.clone()], tokenizer),
            loc,
        }]),
    })
}

pub fn done<ACC: Clone>(tokenizer: &mut Tokenizer, acc: &ACC) -> Result<State<ACC>, Errors> {
    state_op(tokenizer, acc, &|a| State::Done(a))
}

pub fn go_on<ACC: Clone>(tokenizer: &mut Tokenizer, acc: &ACC) -> Result<State<ACC>, Errors> {
    state_op(tokenizer, acc, &|a| State::GoOn(a))
}

fn state_op<ACC: Clone>(
    tokenizer: &mut Tokenizer,
    acc: &ACC,
    op: &impl Fn(ACC) -> State<ACC>,
) -> Result<State<ACC>, Errors> {
    let cloned_acc = acc.clone();
    tokenizer.next();
    Ok(op(cloned_acc))
}

pub fn read_ident(tokenizer: &mut Tokenizer) -> Result<Ident, Errors> {
    shift_err(tokenizer.curr(), |tok| match tok {
        PosTok {
            tok: Tok::Ident(ident),
            ..
        } => {
            tokenizer.next();
            Ok(Ident::new(&ident))
        }
        PosTok { loc, .. } => Err(vec![Error {
            message: expected_msg(&[Tok::Ident("".to_owned())], &tokenizer),
            loc,
        }]),
    })
}

pub fn expected_msg(tokens: &[Tok], tokenizer: &Tokenizer) -> String {
    let expected = tokens
        .iter()
        .take(tokens.len() - 1)
        .fold("".to_owned(), |acc, t| {
            let token_str = match t {
                Tok::Ident(ident) if ident == "" => "identifier".to_owned(),
                Tok::Int(int) if int == "" => "integer".to_owned(),
                _ => format!("{}", t),
            };
            if acc.is_empty() {
                token_str
            } else {
                format!("{}, {}", acc, token_str)
            }
        });
    let expected2 = match tokens.last().unwrap() {
        Tok::Ident(ident) if ident == "" => "identifier".to_owned(),
        Tok::Int(int) if int == "" => "integer".to_owned(),
        _ => format!("{}", tokens.last().unwrap()),
    };
    let expected3 = if expected.is_empty() {
        expected2
    } else {
        format!("{} or {}", expected, expected2)
    };
    format!(
        "Expected: {}\nGot: {}",
        expected3,
        tokenizer.curr().unwrap()
    )
}
