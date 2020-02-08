use parsing::lines_reader::LinesReader;
use parsing::model::*;
use parsing::tokenizer::Tokenizer;
use parsing::tokenizer::{PosTok, Tok};
use parsing::utils::{expected_msg, shift_err, spaces, while_not_done_or_eof, State};
use semantic::model::FuncName;
use std::collections::HashMap;

pub struct AnalyticsParser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> AnalyticsParser<'a> {
    pub fn new(reader: &'a mut LinesReader<'a>) -> Self {
        let tokenizer = Tokenizer::new(reader);
        AnalyticsParser { tokenizer }
    }
    pub fn parse<A: Clone>(
        &mut self,
        analytics_parser: &impl Fn(&mut Tokenizer) -> Result<A, Errors>,
    ) -> Result<HashMap<FuncName, A>, Errors> {
        while_not_done_or_eof(HashMap::new(), |hacc: HashMap<FuncName, A>| {
            spaces(&mut self.tokenizer)?;
            shift_err(self.tokenizer.curr(), |tok| match tok {
                PosTok { tok: Tok::EOF, .. } => Ok(State::Done(hacc.clone())),
                PosTok {
                    tok: Tok::Ident(ident),
                    ..
                } if ident == "legenda" => Ok(State::Done(hacc.clone())),
                PosTok {
                    tok: Tok::NL(_), ..
                } => {
                    self.tokenizer.next();
                    Ok(State::GoOn(hacc.clone()))
                }
                _ => match self.line(analytics_parser) {
                    Ok((f, h)) => {
                        let mut analytics = hacc.clone();
                        analytics.insert(f, h);
                        Ok(State::GoOn(analytics))
                    }
                    Err(errors) => Err(errors),
                },
            })
        })
    }
    fn line<A>(
        &mut self,
        analytics_parser: &impl Fn(&mut Tokenizer) -> Result<A, Errors>,
    ) -> Result<(FuncName, A), Errors> {
        let name = self.name()?;
        self.equals()?;
        let analytics = self.analytics(analytics_parser)?;
        Ok((name, analytics))
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
    fn analytics<A>(
        &mut self,
        analytics_parser: &impl Fn(&mut Tokenizer) -> Result<A, Errors>,
    ) -> Result<A, Errors> {
        spaces(&mut self.tokenizer)?;
        analytics_parser(&mut self.tokenizer)
    }

    fn equals(&mut self) -> Result<(), Errors> {
        spaces(&mut self.tokenizer)?;
        shift_err(self.tokenizer.curr(), |tok| match tok {
            PosTok { tok: Tok::Eq, .. } => {
                self.tokenizer.next();
                Ok(())
            }
            PosTok { loc, .. } => Err(vec![Error {
                message: expected_msg(&[Tok::Eq], &self.tokenizer),
                loc,
            }]),
        })
    }
}

#[cfg(test)]
mod parser {

    #[test]
    fn eof() {}
}
