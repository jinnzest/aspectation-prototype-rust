use parsing::lines_reader::LinesReader;
use parsing::model::*;
use parsing::tokenizer::Tokenizer;
use parsing::tokenizer::{PosTok, Tok};
use parsing::utils::{expected_msg, shift_err, spaces, while_not_done_or_eof, State};
use semantic::model::FuncName;
use std::collections::HashMap;

pub struct HintsParser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> HintsParser<'a> {
    pub fn new(reader: &'a mut LinesReader<'a>) -> Self {
        let tokenizer = Tokenizer::new(reader);
        HintsParser { tokenizer }
    }
    pub fn parse<H: Clone>(
        &mut self,
        hint_parser: &impl Fn(&mut Tokenizer) -> Result<H, Errors>,
    ) -> Result<HashMap<FuncName, H>, Errors> {
        while_not_done_or_eof(HashMap::new(), |hacc: HashMap<FuncName, H>| {
            spaces(&mut self.tokenizer)?;
            shift_err(self.tokenizer.curr(), |tok| match tok {
                PosTok { tok: Tok::EOF, .. } => Ok(State::Done(hacc.clone())),
                PosTok {
                    tok: Tok::NL(_), ..
                } => {
                    self.tokenizer.next();
                    Ok(State::GoOn(hacc.clone()))
                }
                _ => match self.line(hint_parser) {
                    Ok((f, h)) => {
                        let mut hints = hacc.clone();
                        hints.insert(f, h);
                        Ok(State::GoOn(hints))
                    }
                    Err(errors) => Err(errors),
                },
            })
        })
    }
    fn line<H>(
        &mut self,
        hint_parser: &impl Fn(&mut Tokenizer) -> Result<H, Errors>,
    ) -> Result<(FuncName, H), Errors> {
        let name = self.name()?;
        self.arrow_left()?;
        let hint = self.hint(hint_parser)?;
        Ok((name, hint))
    }
    fn name(&mut self) -> Result<FuncName, Errors> {
        spaces(&mut self.tokenizer)?;
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok {
                tok: Tok::Ident(val),
                ..
            } => {
                self.tokenizer.next();
                Ok(FuncName::new(&val))
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::Ident("".to_owned())], &self.tokenizer),
                loc,
            }]),
        })
    }
    fn hint<H>(
        &mut self,
        hint_parser: &impl Fn(&mut Tokenizer) -> Result<H, Errors>,
    ) -> Result<H, Errors> {
        spaces(&mut self.tokenizer)?;
        hint_parser(&mut self.tokenizer)
    }

    fn arrow_left(&mut self) -> Result<(), Errors> {
        spaces(&mut self.tokenizer)?;
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok {
                tok: Tok::ArwLft, ..
            } => {
                self.tokenizer.next();
                Ok(())
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::ArwLft], &self.tokenizer),
                loc,
            }]),
        })
    }
}

#[cfg(test)]
mod side_effect_parser {

    #[test]
    fn eof() {}
}
