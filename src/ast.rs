use std::{collections::HashMap, fmt::Write, mem};

use crate::{err::Result, writer::FakeWriter};

pub enum Expr {
    None,
    Variable(Variable),
    Literal(Literal),
    Concat(Concat),
    Condition(Condition),
}

pub struct Variable(String);
pub struct Literal(String);
pub struct Concat(Vec<Expr>);

pub struct Condition {
    cond: Box<Expr>,
    success: Box<Expr>,
    failure: Box<Expr>,
}

impl Expr {
    pub fn eval<W>(
        self,
        res: &mut W,
        vars: &HashMap<String, String>,
    ) -> Result<bool>
    where
        W: Write,
    {
        match self {
            Self::None => Ok(false),
            Self::Variable(v) => v.eval(res, vars).map(|a| a.into()),
            Self::Literal(l) => l.eval(res),
            Self::Concat(c) => c.eval(res, vars),
            Self::Condition(c) => c.eval(res, vars),
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

impl From<Condition> for Expr {
    fn from(value: Condition) -> Self {
        Self::Condition(value)
    }
}

impl Variable {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn eval<W>(
        self,
        res: &mut W,
        vars: &HashMap<String, String>,
    ) -> Result<bool>
    where
        W: Write,
    {
        if let Some(v) = vars.get(&self.0) {
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

    pub fn eval<W>(
        self,
        res: &mut W,
        vars: &HashMap<String, String>,
    ) -> Result<bool>
    where
        W: Write,
    {
        self.0
            .into_iter()
            .map(|e| e.eval(res, vars))
            .try_fold(true, |a, b| Ok(a | b?))
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

    pub fn eval<W>(
        self,
        res: &mut W,
        vars: &HashMap<String, String>,
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
