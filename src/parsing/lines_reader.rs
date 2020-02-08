use parsing::model::Loc;
use std::str::CharIndices;

pub struct LinesReader<'a> {
    input: &'a str,
    line_pos: Vec<usize>,
    chars: CharIndices<'a>,
    curr_char: Option<char>,
    mark: Loc,
    curr: Loc,
}

impl<'a> LinesReader<'a> {
    pub fn new(input: &str) -> LinesReader {
        let mut reader = LinesReader {
            input,
            line_pos: vec![],
            chars: input.char_indices(),
            curr_char: None,
            mark: Loc {
                pos: 0,
                line: 1,
                col: 1,
            },
            curr: Loc {
                pos: 0,
                line: 1,
                col: 1,
            },
        };
        reader.line_pos.push(0);
        reader.next();
        reader.mark_range();
        reader
    }
    pub fn curr_char(&self) -> Option<char> {
        self.curr_char
    }
    pub fn mark_loc(&self) -> Loc {
        self.mark
    }
    pub fn curr_loc(&self) -> Loc {
        self.curr
    }
    pub fn next(&mut self) {
        let last_char = self.curr_char;
        match self.chars.next() {
            Some((pos, ch)) => self.set_curr(last_char, pos, ch),
            None => {
                self.curr_char = None;
                self.curr.pos += 1;
                self.curr.col += 1;
            }
        }
    }
    fn set_curr(&mut self, last_char: Option<char>, pos: usize, ch: char) {
        self.curr_char = Some(ch);
        if ch == '\n' {
            self.set_curr_nl(pos);
        } else {
            self.set_curr_non_nl(last_char, pos);
        }
    }
    fn set_curr_non_nl(&mut self, last_char: Option<char>, pos: usize) {
        let diff = match last_char {
            Some('\n') => 0,
            _ => pos - self.curr.pos,
        };
        self.curr.pos = pos;
        self.curr.col += diff;
    }
    fn set_curr_nl(&mut self, pos: usize) {
        self.curr.pos = pos;
        self.curr.line += 1;
        self.curr.col = 1;
        self.line_pos.push(pos);
    }
    pub fn line_pos(&self) -> Vec<usize> {
        self.line_pos.clone()
    }
    pub fn mark_range(&mut self) {
        self.mark = self.curr;
    }
    pub fn extract_range(&self) -> &str {
        if self.curr.pos > self.input.len() {
            &self.input[self.mark.pos..self.curr.pos - 1]
        } else {
            &self.input[self.mark.pos..self.curr.pos]
        }
    }
}

#[cfg(test)]
mod lines_reader {
    use super::*;

    #[test]
    fn eof() {
        let reader = LinesReader::new("");
        assert_eq!(reader.curr_char(), None);
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 1,
                line: 1,
                col: 2,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 1,
                col: 2,
            }
        );
    }

    #[test]
    fn one_space_char() {
        let reader = LinesReader::new("1");
        assert_eq!(reader.curr_char(), Some('1'));
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
    }

    #[test]
    fn one_new_line() {
        let reader = LinesReader::new("\n");
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
    }

    #[test]
    fn chars_and_new_lines() {
        let mut reader = LinesReader::new("1\n2\n34\n5\n6\n\n7");
        assert_eq!(reader.curr_char(), Some('1'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 2,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('2'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 2,
                line: 2,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 3,
                line: 3,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('3'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 4,
                line: 3,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('4'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 5,
                line: 3,
                col: 2,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 6,
                line: 4,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('5'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 7,
                line: 4,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 8,
                line: 5,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('6'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 9,
                line: 5,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 10,
                line: 6,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 11,
                line: 7,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('7'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 12,
                line: 7,
                col: 1,
            }
        );
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 1,
                col: 1,
            }
        );
    }

    #[test]
    fn new_lines_and_chars() {
        let mut reader = LinesReader::new("\n1\n2\n34\n5\n6\n\n7\n");
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('1'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 2,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 2,
                line: 3,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('2'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 3,
                line: 3,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 4,
                line: 4,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('3'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 5,
                line: 4,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('4'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 6,
                line: 4,
                col: 2,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 7,
                line: 5,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('5'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 8,
                line: 5,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 9,
                line: 6,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('6'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 10,
                line: 6,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 11,
                line: 7,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 12,
                line: 8,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('7'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 13,
                line: 8,
                col: 1,
            }
        );
        reader.next();
        assert_eq!(reader.curr_char(), Some('\n'));
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 14,
                line: 9,
                col: 1,
            }
        );
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
    }

    #[test]
    fn line_pos() {
        let mut reader = LinesReader::new("1\n12\n12\n1234\n1");
        while reader.curr_char.is_some() {
            reader.next();
        }
        assert_eq!(reader.line_pos(), vec![0, 1, 4, 7, 12]);
    }

    #[test]
    fn extract_ranges() {
        let mut reader = LinesReader::new("1\n\n1234\n1");
        reader.next(); //move to \n
        let range = reader.extract_range();
        assert_eq!("1", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 2,
                col: 1,
            }
        );
        reader.mark_range();
        reader.next(); //move to \n
        reader.next(); //move to 1
        let range = reader.extract_range();
        assert_eq!("\n\n", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 3,
                line: 3,
                col: 1,
            }
        );
        reader.mark_range();
        while reader.curr_char().unwrap().is_digit(10) {
            reader.next();
        }
        let range = reader.extract_range();
        assert_eq!("1234", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 7,
                line: 4,
                col: 1,
            }
        );
        reader.next(); //move to 1
        reader.mark_range();
        reader.next(); //move to EOF
        let range = reader.extract_range();
        assert_eq!("1", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 9,
                line: 4,
                col: 2,
            }
        );
    }

    #[test]
    fn extract_nl_range() {
        let mut reader = LinesReader::new("\n");
        reader.next();
        let range = reader.extract_range();
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 2,
                col: 2,
            }
        );
        assert_eq!("\n", range);
    }

    #[test]
    fn extract_2nl_range() {
        let mut reader = LinesReader::new("\n\n");
        reader.next();
        let range = reader.extract_range();
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 3,
                col: 1,
            }
        );
        assert_eq!("\n", range);
        reader.mark_range();
        reader.next();
        let range = reader.extract_range();
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 1,
                line: 3,
                col: 1,
            }
        );
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 2,
                line: 3,
                col: 2,
            }
        );
        assert_eq!("\n", range);
    }

    #[test]
    fn extract_2nd_line_range() {
        let mut reader = LinesReader::new("\n12\n");
        reader.next();
        assert_eq!(
            reader.mark_loc(),
            Loc {
                pos: 0,
                line: 2,
                col: 1,
            }
        );
        reader.mark_range();
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 1,
                line: 2,
                col: 1,
            }
        );
        reader.next(); //move to spaces
        reader.next(); //move to last \n
        let range = reader.extract_range();
        assert_eq!("12", range);
    }

    #[test]
    fn extract_range_eof() {
        let mut reader = LinesReader::new("a\n\n1234");
        while !reader.curr_char().unwrap().is_digit(10) {
            reader.next();
        }
        reader.mark_range();
        while reader.curr_char().is_some() {
            reader.next();
        }
        let range = reader.extract_range();
        assert_eq!("1234", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 7,
                line: 3,
                col: 5,
            }
        )
    }

    #[test]
    fn extract_range_begin() {
        let mut reader = LinesReader::new("1234\n \n ");
        while reader.curr_char().is_some() && reader.curr_char().unwrap().is_digit(10) {
            reader.next();
        }
        let range = reader.extract_range();
        assert_eq!("1234", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 4,
                line: 2,
                col: 1,
            }
        );
        reader.next(); //move to space
        reader.next(); //move to new line
        reader.next(); //move to new line
        assert_eq!(' ', reader.curr_char().unwrap());
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 7,
                line: 3,
                col: 1,
            }
        );
    }

    #[test]
    fn extract_range_after_eof() {
        let mut reader = LinesReader::new("a");
        reader.mark_range();
        reader.next();
        reader.next();
        let range = reader.extract_range();
        assert_eq!("a", range);
        assert_eq!(
            reader.curr_loc(),
            Loc {
                pos: 2,
                line: 1,
                col: 3,
            }
        )
    }
}
