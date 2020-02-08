use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Loc {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Range {
    pub begin: Loc,
    pub end: Loc,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ranged<T> {
    pub v: T,
    pub range: Range,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub loc: Loc,
    pub message: String,
}

pub type Errors = Vec<Error>;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Ord, PartialOrd)]
pub struct Ident(String);

impl Ident {
    pub fn new(n: &str) -> Self {
        Ident(n.to_owned())
    }
    pub fn str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.str())
    }
}
