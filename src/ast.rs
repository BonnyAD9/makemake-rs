use std::{borrow::Cow, collections::HashMap, fmt::Write, mem};

use crate::{err::Result, writer::FakeWriter};

pub enum Expr {
    None,
    Variable(Variable),
    Literal(Literal),
    Concat(Concat),
    Equals(Equals),
    Condition(Condition),
    NullCheck(NullCheck),
}

pub struct Variable(String);
pub struct Literal(String);
pub struct Concat(Vec<Expr>);
pub struct Equals(Box<Expr>, Box<Expr>);

pub struct Condition {
    cond: Box<Expr>,
    success: Box<Expr>,
    failure: Box<Expr>,
}

pub struct NullCheck {
    cond: Box<Expr>,
    other: Box<Expr>,
}

impl Expr {
    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        match self {
            Self::None => Ok(false),
            Self::Variable(v) => v.eval(res, vars),
            Self::Literal(l) => l.eval(res),
            Self::Concat(c) => c.eval(res, vars),
            Self::Equals(e) => e.eval(res, vars),
            Self::Condition(c) => c.eval(res, vars),
            Self::NullCheck(n) => n.eval(res, vars),
        }
    }

    pub fn concat(&mut self, other: Expr) {
        match self {
            Self::None => *self = other,
            Self::Concat(c) => c.0.push(other),
            _ => {
                let tmp = mem::replace(self, Self::None);
                *self = Expr::Concat(Concat::new(vec![tmp, other]));
            }
        }
    }
}

impl From<Variable> for Expr {
    fn from(value: Variable) -> Self {
        Self::Variable(value)
    }
}

impl From<Literal> for Expr {
    fn from(value: Literal) -> Self {
        Self::Literal(value)
    }
}

impl From<Concat> for Expr {
    fn from(value: Concat) -> Self {
        Self::Concat(value)
    }
}

impl From<Equals> for Expr {
    fn from(value: Equals) -> Self {
        Self::Equals(value)
    }
}

impl From<Condition> for Expr {
    fn from(value: Condition) -> Self {
        Self::Condition(value)
    }
}

impl From<NullCheck> for Expr {
    fn from(value: NullCheck) -> Self {
        Self::NullCheck(value)
    }
}

impl Variable {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        if let Some(v) = vars.get(&Cow::Owned(self.0)) {
            res.write_str(v)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Literal {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn eval<W>(self, res: &mut W) -> Result<bool>
    where
        W: Write,
    {
        res.write_str(&self.0)?;
        Ok(true)
    }
}

impl Concat {
    pub fn new(exprs: Vec<Expr>) -> Self {
        Self(exprs)
    }

    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        self.0
            .into_iter()
            .map(|e| e.eval(res, vars))
            .try_fold(false, |a, b| Ok(a | b?))
    }
}

impl Equals {
    pub fn new(l: Expr, r: Expr) -> Self {
        Self(Box::new(l), Box::new(r))
    }

    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        let mut l = String::new();
        let mut r = String::new();
        let lres = self.0.eval(&mut l, vars)?;
        let rres = self.1.eval(&mut r, vars)?;

        if lres == rres && l == r {
            res.write_str(&l)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Condition {
    pub fn new(cond: Expr, success: Expr, failure: Expr) -> Self {
        Self {
            cond: Box::new(cond),
            success: Box::new(success),
            failure: Box::new(failure),
        }
    }

    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        let mut w = FakeWriter;
        if self.cond.eval(&mut w, vars)? {
            self.success.eval(res, vars)
        } else {
            self.failure.eval(res, vars)
        }
    }
}

impl NullCheck {
    pub fn new(cond: Expr, other: Expr) -> Self {
        Self {
            cond: Box::new(cond),
            other: Box::new(other),
        }
    }

    pub fn eval<'a, W>(
        self,
        res: &mut W,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<bool>
    where
        W: Write,
    {
        let mut w = String::new();
        if self.cond.eval(&mut w, vars)? {
            res.write_str(&w)?;
            Ok(true)
        } else {
            self.other.eval(res, vars)
        }
    }
}
