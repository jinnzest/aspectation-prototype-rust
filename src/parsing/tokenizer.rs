use parsing::lines_reader::LinesReader;
use parsing::model::{Error, Errors, Loc};
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Tok {
    Int(String),
    NL(usize),
    Sps(usize),
    Ident(String),
    Fn,
    Eq,
    OpnBkt,
    ClsBkt,
    EOF,
    Minus,
    ArwLft,
    Comma,
    Colon,
    Less,
}

pub fn ident(s: &str) -> Tok {
    Tok::Ident(s.to_owned())
}

#[derive(Debug)]
enum TokState {
    Int,
    Sps,
    Ident,
    ArrowLeft,
    None,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PosTok {
    pub tok: Tok,
    pub loc: Loc,
}

pub struct Tokenizer<'a> {
    pub reader: &'a mut LinesReader<'a>,
    curr_tok: Option<Result<PosTok, Errors>>,
    tok_state: TokState,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorCode {
    UnexpectedChar(char),
    UnexpectedEnd,
}

impl<'a> Tokenizer<'a> {
    pub fn new(reader: &'a mut LinesReader<'a>) -> Tokenizer<'a> {
        let mut tokenizer = Tokenizer {
            reader,
            curr_tok: None,
            tok_state: TokState::None,
        };
        tokenizer.next();
        tokenizer
    }
    pub fn curr(&self) -> Result<PosTok, Errors> {
        self.curr_tok.clone().unwrap()
    }
    pub fn next(&mut self) {
        self.curr_tok = None;
        while self.curr_tok.is_none() {
            self.match_state()
        }
    }
    fn match_state(&mut self) {
        //        println!("{:?}, '{}'", self.tok_state, c);
        match self.tok_state {
            TokState::None => self.match_none_state(),
            TokState::Ident => {
                if self.is_alphanumeric() {
                    self.reader.next();
                } else {
                    self.match_word();
                }
            }
            TokState::Int => {
                if self.is_digit() {
                    self.reader.next();
                } else {
                    self.match_integer();
                }
            }
            TokState::Sps => {
                if self.curr_char_is(' ') {
                    self.reader.next();
                } else {
                    self.match_spaces();
                }
            }
            TokState::ArrowLeft => {
                if self.curr_char_is('-') {
                    self.match_arrow_left();
                    self.reader.next();
                } else {
                    self.match_less();
                }
            }
        }
    }
    fn match_none_state(&mut self) {
        match self.reader.curr_char() {
            None => {
                self.curr_tok = Some(Ok(PosTok {
                    tok: Tok::EOF,
                    loc: self.reader.curr_loc(),
                }));
                self.reader.mark_range();
            }
            Some(c) => self.match_char(c),
        }
    }
    fn match_char(&mut self, c: char) {
        match c {
            '\n' => self.handle_new_line_tok(),
            '=' => self.handle_one_char_tok(&|_| Tok::Eq),
            '(' => self.handle_one_char_tok(&|_| Tok::OpnBkt),
            ')' => self.handle_one_char_tok(&|_| Tok::ClsBkt),
            ',' => self.handle_one_char_tok(&|_| Tok::Comma),
            ':' => self.handle_one_char_tok(&|_| Tok::Colon),
            '-' => self.handle_one_char_tok(&|_| Tok::Minus),
            ' ' => self.next_and_switch_state(TokState::Sps),
            '<' => self.next_and_switch_state(TokState::ArrowLeft),
            _ if is_ident_begin(c) => self.next_and_switch_state(TokState::Ident),
            _ if c.is_digit(10) => self.next_and_switch_state(TokState::Int),
            unexpected => self.handle_error(unexpected),
        }
    }
    fn next_and_switch_state(&mut self, state: TokState) {
        //        println!("{:?}", state);
        self.tok_state = state;
        self.reader.mark_range();
        self.reader.next();
    }
    fn handle_new_line_tok(&mut self) {
        self.handle_one_char_tok(&|len| Tok::NL(len));
    }
    fn handle_error(&mut self, unexpected: char) {
        self.curr_tok = Some(Err(vec![Error {
            message: format!("{}", ErrorCode::UnexpectedChar(unexpected)),
            loc: self.reader.curr_loc(),
        }]))
    }
    fn curr_char_is(&self, ch: char) -> bool {
        //        println!(" '{:?}' == '{}'", self.reader.curr_char(), ch);
        match self.reader.curr_char() {
            Some(c) => c == ch,
            None => false,
        }
    }
    fn handle_one_char_tok(&mut self, f: &impl Fn(usize) -> Tok) {
        self.reader.next();
        self.match_range(&|range| f(range.len()));
    }
    fn match_spaces(&mut self) {
        self.match_range(&|spaces| Tok::Sps(spaces.len()));
    }
    fn match_word(&mut self) {
        self.match_range(&|word| match word {
            "fn" => Tok::Fn,
            id => Tok::Ident(id.to_owned()),
        });
    }
    fn match_integer(&mut self) {
        self.match_range(&|int| Tok::Int(int.to_owned()));
    }
    fn match_range(&mut self, f: &impl Fn(&str) -> Tok) {
        self.tok_state = TokState::None;
        let range = self.reader.extract_range();
        self.curr_tok = Some(Ok(PosTok {
            tok: f(range),
            loc: self.reader.mark_loc(),
        }));
        self.reader.mark_range();
    }
    fn match_arrow_left(&mut self) {
        self.tok_state = TokState::None;
        self.curr_tok = Some(Ok(PosTok {
            tok: Tok::ArwLft,
            loc: self.reader.mark_loc(),
        }));
    }
    fn match_less(&mut self) {
        self.tok_state = TokState::None;
        self.curr_tok = Some(Ok(PosTok {
            tok: Tok::Less,
            loc: self.reader.mark_loc(),
        }));
    }
    fn is_digit(&self) -> bool {
        match self.reader.curr_char() {
            Some(c) => c.is_digit(10),
            None => false,
        }
    }
    fn is_alphanumeric(&self) -> bool {
        match self.reader.curr_char() {
            Some(c) => c.is_alphanumeric() || c == '_',
            None => false,
        }
    }
    pub fn line_pos(&self) -> Vec<usize> {
        self.reader.line_pos()
    }
}

fn is_ident_begin(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

impl<'a> fmt::Display for PosTok {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {}", self.tok, self.loc)
    }
}

impl<'a> fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'")?;
        match self {
            Tok::Eq => write!(f, "="),
            Tok::EOF => write!(f, "EOF"),
            Tok::OpnBkt => write!(f, "("),
            Tok::ClsBkt => write!(f, ")"),
            Tok::Int(s) => write!(f, "{}", s),
            Tok::Ident(s) => write!(f, "{}", s),
            Tok::Sps(s) => write!(
                f,
                "{}",
                (0..*s).map({ |_| " " }).collect::<Vec<_>>().concat()
            ),
            Tok::NL(_) => write!(f, "new line"),
            Tok::ArwLft => write!(f, "<-"),
            Tok::Comma => write!(f, ","),
            Tok::Colon => write!(f, ":"),
            Tok::Minus => write!(f, "-"),
            Tok::Fn => write!(f, "fn"),
            Tok::Less => write!(f, "<"),
        }?;
        write!(f, "'")
    }
}

impl<'a> fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

impl<'a> fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorCode::UnexpectedChar(ch) => write!(f, "unexpected char: '{}'", ch),
            ErrorCode::UnexpectedEnd => write!(f, "unexpected end"),
        }
    }
}

#[cfg(test)]
mod tokenizer {
    use super::*;

    #[test]
    fn initial_curr_tok() {
        let mut reader = LinesReader::new("");
        let tokenizer = Tokenizer::new(&mut reader);
        assert_eq!(
            tokenizer.curr().ok().unwrap(),
            PosTok {
                tok: Tok::EOF,
                loc: Loc {
                    pos: 1,
                    line: 1,
                    col: 2,
                },
            }
        );
    }

    #[test]
    fn new_line_only() {
        let mut reader = LinesReader::new("\n");
        let tokenizer = Tokenizer::new(&mut reader);
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(
            value,
            PosTok {
                tok: Tok::NL(1),
                loc: Loc {
                    pos: 0,
                    line: 2,
                    col: 1,
                },
            }
        );
    }

    #[test]
    fn new_lines_count() {
        let mut reader = LinesReader::new("\n\n\n  \n\n");
        let mut tokenizer = Tokenizer::new(&mut reader);
        assert_tok_and_pos(
            Tok::NL(1),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            },
            tokenizer.curr(),
        );
        tokenizer.next();
        assert_tok_and_pos(
            Tok::NL(1),
            Loc {
                pos: 1,
                line: 3,
                col: 1,
            },
            tokenizer.curr(),
        );
        tokenizer.next();
        assert_tok_and_pos(
            Tok::NL(1),
            Loc {
                pos: 2,
                line: 4,
                col: 1,
            },
            tokenizer.curr(),
        );
        tokenizer.next();
        assert_tok_and_pos(
            Tok::Sps(2),
            Loc {
                pos: 3,
                line: 4,
                col: 1,
            },
            tokenizer.curr(),
        );
        tokenizer.next();
        assert_tok_and_pos(
            Tok::NL(1),
            Loc {
                pos: 5,
                line: 5,
                col: 1,
            },
            tokenizer.curr(),
        );
        tokenizer.next();
        assert_tok_and_pos(
            Tok::NL(1),
            Loc {
                pos: 6,
                line: 6,
                col: 1,
            },
            tokenizer.curr(),
        );
    }

    fn assert_tok_and_pos(tok: Tok, loc: Loc, value_res: Result<PosTok, Errors>) {
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, tok);
        assert_eq!(value.loc, loc);
    }

    #[test]
    fn fn_only() {
        let mut reader = LinesReader::new("fn");
        let tokenizer = Tokenizer::new(&mut reader);
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Fn);
        assert_eq!(
            value.loc,
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
    }

    #[test]
    fn fn_only_spaces() {
        let mut reader = LinesReader::new(" fn  ");
        let mut tokenizer = Tokenizer::new(&mut reader);
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::Sps(1));
        assert_eq!(
            tokenizer.curr().ok().unwrap().loc,
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
        tokenizer.next();
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::Fn);
        assert_eq!(
            tokenizer.curr().ok().unwrap().loc,
            Loc {
                pos: 1,
                line: 1,
                col: 2,
            }
        );
        tokenizer.next();
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::Sps(2));
        assert_eq!(
            tokenizer.curr().ok().unwrap().loc,
            Loc {
                pos: 3,
                line: 1,
                col: 4,
            }
        );
        tokenizer.next();
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::EOF);
        assert_eq!(
            tokenizer.curr().ok().unwrap().loc,
            Loc {
                pos: 5,
                line: 1,
                col: 6,
            }
        );
    }

    #[test]
    fn fn_only_next_line() {
        let mut reader = LinesReader::new("  \n fn");
        let mut tokenizer = Tokenizer::new(&mut reader);
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let space = value_res.ok().unwrap();
        assert_eq!(space.tok, Tok::Sps(2));
        assert_eq!(
            space.loc,
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
        tokenizer.next();
        let new_line_res = tokenizer.curr();
        assert!(new_line_res.is_ok());
        let new_line = new_line_res.ok().unwrap();
        assert_eq!(new_line.tok, Tok::NL(1));
        assert_eq!(
            new_line.loc,
            Loc {
                pos: 2,
                line: 2,
                col: 1,
            }
        );
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Sps(1));
        assert_eq!(
            value.loc,
            Loc {
                pos: 3,
                line: 2,
                col: 1,
            }
        );
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Fn);
        assert_eq!(
            value.loc,
            Loc {
                pos: 4,
                line: 2,
                col: 2,
            }
        );
    }

    #[test]
    fn integer() {
        let mut reader = LinesReader::new("  \n\n   123 ");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read new line
        tokenizer.next(); //read new line
        tokenizer.next(); //read spaces
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Int("123".to_owned()));
        assert_eq!(
            value.loc,
            Loc {
                pos: 7,
                line: 3,
                col: 4,
            }
        );
    }

    #[test]
    fn integers() {
        let mut reader = LinesReader::new(" 1 2 345 ");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read number
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::Int("1".to_owned()));
        tokenizer.next(); //read spaces
        tokenizer.next(); //read number
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::Int("2".to_owned()));
        tokenizer.next(); //read spaces
        tokenizer.next(); //read number
        assert_eq!(
            tokenizer.curr().ok().unwrap().tok,
            Tok::Int("345".to_owned())
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::EOF);
        assert_eq!(
            tokenizer.curr().ok().unwrap().loc,
            Loc {
                pos: 9,
                line: 1,
                col: 10
            }
        );
    }

    #[test]
    fn integers_and_idents() {
        let mut reader = LinesReader::new(" 1 a 2 bc 3 d 45 e");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read number
        assert_eq!(Tok::Int("1".to_owned()), tokenizer.curr().ok().unwrap().tok);
        tokenizer.next(); //read spaces
        tokenizer.next(); //read number
        assert_eq!(
            Tok::Ident("a".to_owned()),
            tokenizer.curr().ok().unwrap().tok
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read number
        assert_eq!(Tok::Int("2".to_owned()), tokenizer.curr().ok().unwrap().tok);
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(
            Tok::Ident("bc".to_owned()),
            tokenizer.curr().ok().unwrap().tok
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(Tok::Int("3".to_owned()), tokenizer.curr().ok().unwrap().tok);
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(
            Tok::Ident("d".to_owned()),
            tokenizer.curr().ok().unwrap().tok
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(
            Tok::Int("45".to_owned()),
            tokenizer.curr().ok().unwrap().tok
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(
            Tok::Ident("e".to_owned()),
            tokenizer.curr().ok().unwrap().tok
        );
        tokenizer.next(); //read spaces
        tokenizer.next(); //read eof
        assert_eq!(Tok::EOF, tokenizer.curr().ok().unwrap().tok);
    }

    #[test]
    fn integer_eof() {
        let mut reader = LinesReader::new("  \n\n   123");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read new line
        tokenizer.next(); //read new line
        tokenizer.next(); //read spaces
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Int("123".to_owned()));
        assert_eq!(
            value.loc,
            Loc {
                pos: 7,
                line: 3,
                col: 4,
            }
        );
    }

    #[test]
    fn identifier_eof() {
        let mut reader = LinesReader::new(" \n  some_identifier1");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read new line
        tokenizer.next(); //read spaces
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Ident("some_identifier1".to_owned()));
        assert_eq!(
            value.loc,
            Loc {
                pos: 4,
                line: 2,
                col: 3,
            }
        );
        tokenizer.next();
        assert_eq!(
            tokenizer.curr(),
            Ok(PosTok {
                tok: Tok::EOF,
                loc: Loc {
                    pos: 20,
                    line: 2,
                    col: 19,
                },
            })
        );
    }

    #[test]
    fn identifier() {
        let mut reader = LinesReader::new(" \n  some_identifier1  ");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next(); //read new line
        tokenizer.next(); //read spaces
        tokenizer.next();
        let value_res = tokenizer.curr();
        assert!(value_res.is_ok());
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::Ident("some_identifier1".to_owned()));
        assert_eq!(
            value.loc,
            Loc {
                pos: 4,
                line: 2,
                col: 3,
            }
        );
        tokenizer.next();
        tokenizer.next(); //read spaces
        let value_res = tokenizer.curr();
        let value = value_res.ok().unwrap();
        assert_eq!(value.tok, Tok::EOF);
        assert_eq!(
            value.loc,
            Loc {
                pos: 22,
                line: 2,
                col: 21,
            }
        );
    }

    #[test]
    fn parse_all_identifiers() {
        let mut reader = LinesReader::new(" fn name arg1 arg2 = body1 body2 ");
        let mut tokenizer = Tokenizer::new(&mut reader);
        let mut not_done = true;
        let mut tokens = vec![];
        while not_done {
            match tokenizer.curr() {
                Ok(PosTok { tok: Tok::EOF, .. }) => not_done = false,
                Ok(PosTok { tok, .. }) => tokens.push(tok),
                _ => {}
            }
            tokenizer.next();
        }
        assert_eq!(tokenizer.curr().ok().unwrap().tok, Tok::EOF);
        assert_eq!(
            tokens,
            vec![
                Tok::Sps(1),
                Tok::Fn,
                Tok::Sps(1),
                Tok::Ident("name".to_owned()),
                Tok::Sps(1),
                Tok::Ident("arg1".to_owned()),
                Tok::Sps(1),
                Tok::Ident("arg2".to_owned()),
                Tok::Sps(1),
                Tok::Eq,
                Tok::Sps(1),
                Tok::Ident("body1".to_owned()),
                Tok::Sps(1),
                Tok::Ident("body2".to_owned()),
                Tok::Sps(1)
            ]
        );
    }

    #[test]
    fn line_pos() {
        let mut reader = LinesReader::new(" \n  some_identifier1  \n   \n1234 \n \n   \nh");
        let mut tokenizer = Tokenizer::new(&mut reader);
        let mut next = tokenizer.curr();
        while next.is_ok() && next.ok().unwrap().tok != Tok::EOF {
            tokenizer.next();
            next = tokenizer.curr();
        }
        assert_eq!(tokenizer.line_pos()[0], 0);
        assert_eq!(tokenizer.line_pos()[1], 1);
        assert_eq!(tokenizer.line_pos()[2], 22);
        assert_eq!(tokenizer.line_pos()[3], 26);
    }

    #[test]
    fn minus() {
        let mut reader = LinesReader::new("1-2");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next();
        let minus = tokenizer.curr();
        tokenizer.next();
        assert_eq!(
            minus.ok().unwrap(),
            PosTok {
                tok: Tok::Minus,
                loc: Loc {
                    pos: 1,
                    line: 1,
                    col: 2,
                },
            }
        );
        assert_eq!(
            tokenizer.curr().ok().unwrap(),
            PosTok {
                tok: Tok::Int("2".to_owned()),
                loc: Loc {
                    pos: 2,
                    line: 1,
                    col: 3,
                },
            }
        );
    }
    #[test]
    fn arrow_left() {
        let mut reader = LinesReader::new("1<-2");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next();
        let arrow = tokenizer.curr();
        tokenizer.next();
        assert_eq!(
            arrow.ok().unwrap(),
            PosTok {
                tok: Tok::ArwLft,
                loc: Loc {
                    pos: 1,
                    line: 1,
                    col: 2,
                },
            }
        );
        let number = tokenizer.curr();
        assert_eq!(
            number.ok().unwrap(),
            PosTok {
                tok: Tok::Int("2".to_owned()),
                loc: Loc {
                    pos: 3,
                    line: 1,
                    col: 4,
                },
            }
        );
    }
    #[test]
    fn comma() {
        let mut reader = LinesReader::new("1,2");
        let mut tokenizer = Tokenizer::new(&mut reader);
        tokenizer.next();
        let comma = tokenizer.curr();
        tokenizer.next();
        assert_eq!(
            comma.ok().unwrap(),
            PosTok {
                tok: Tok::Comma,
                loc: Loc {
                    pos: 1,
                    line: 1,
                    col: 2,
                },
            }
        );
        assert_eq!(
            tokenizer.curr().ok().unwrap(),
            PosTok {
                tok: Tok::Int("2".to_owned()),
                loc: Loc {
                    pos: 2,
                    line: 1,
                    col: 3,
                },
            }
        );
    }
}
