use aspects::model::*;
use parsing::model::Ident;
use semantic::utils::str_by_comma;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Error, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncName(pub String);

impl FuncName {
    pub fn new(name: &str) -> Self {
        FuncName(name.to_owned())
    }
    pub fn new_from_ident(name: &Ident) -> Self {
        FuncName(name.str().to_owned())
    }
    pub fn str(&self) -> &str {
        &self.0
    }

    pub fn to_owned(&self) -> String {
        self.0.to_owned()
    }
}

impl PartialOrd for FuncName {
    fn partial_cmp(&self, other: &FuncName) -> Option<Ordering> {
        Some(other.0.cmp(&self.0))
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: FuncName,
    pub args: Vec<Ident>,
    pub body: Expression,
}

#[derive(Debug)]
pub enum Construction {
    Function(Function),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: FuncName,
    pub args: Vec<Ident>,
}

#[derive(Debug, Clone)]
pub struct FunctionCallSignature {
    pub name: FuncName,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    FunctionCall(FunctionCallSignature),
    Constant(Ident),
    FunctionArgument(Ident),
    SubExpression(Vec<Expression>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticHash(pub String);

impl SemanticHash {
    pub fn new(hash: &str) -> Self {
        SemanticHash(hash.to_owned())
    }
    pub fn str(&self) -> String {
        self.0.clone()
    }
}

impl<'a> Display for FuncName {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.str())
    }
}

impl Display for SemanticHash {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct HashWithFunc {
    pub hash: SemanticHash,
    pub function: Rc<FunctionSignature>,
}

impl Hash for HashWithFunc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct FnWithHints {
    pub sig: Rc<FunctionSignature>,
    pub body: Expression,
    pub hints: HintFields,
}

#[derive(Debug, Clone)]
pub struct FnWithAnalytics {
    pub sig: Rc<FunctionSignature>,
    pub analytics: AnalyticsFields,
}

#[derive(Debug, Clone)]
pub struct FnWithAspects {
    pub sig: Rc<FunctionSignature>,
    pub hints: HintFields,
    pub analytics: AnalyticsFields,
}

impl<'a> fmt::Display for FunctionCallSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.name,
            self.args.iter().fold("".to_owned(), |acc, e| {
                if acc.is_empty() {
                    format!("{}", e)
                } else {
                    format!("{}, {}", acc, e)
                }
            })
        )
    }
}

impl<'a> fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Expression::Constant(c) => format!("{}", c),
                Expression::FunctionArgument(a) => format!("{}", a),
                Expression::FunctionCall(fs) => format!("{}", fs),
                Expression::SubExpression(se) => str_by_comma(se),
            }
        )
    }
}
