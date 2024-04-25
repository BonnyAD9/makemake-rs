use std::{
    borrow::Cow, collections::HashMap, fmt::Write, fs::File, io::BufReader,
    mem,
};

use utf8_chars::BufReadCharsExt;

use crate::{
    err::{Error, Result},
    maker::{expand, ExpandContext},
    writer::FakeWriter,
};

pub enum Expr {
    None,
    Variable(Variable),
    Literal(Literal),
    Concat(Concat),
    Equals(Equals),
    Condition(Condition),
    NullCheck(NullCheck),
    Call(Call),
}

#[derive(Hash, PartialEq, Eq)]
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

pub struct Call {
    typ: Variable,
    file: Box<Expr>,
    define: HashMap<Variable, Expr>,
    undefine: Vec<Variable>,
}

impl Expr {
    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        match self {
            Self::None => Ok(false),
            Self::Variable(v) => v.eval(res, ctx),
            Self::Literal(l) => l.eval(res),
            Self::Concat(c) => c.eval(res, ctx),
            Self::Equals(e) => e.eval(res, ctx),
            Self::Condition(c) => c.eval(res, ctx),
            Self::NullCheck(n) => n.eval(res, ctx),
            Self::Call(c) => c.eval(res, ctx),
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

impl From<Call> for Expr {
    fn from(value: Call) -> Self {
        Self::Call(value)
    }
}

impl Variable {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        if let Some(v) = ctx.vars.get(&Cow::Owned(self.0)) {
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

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        self.0
            .into_iter()
            .map(|e| e.eval(res, ctx))
            .try_fold(false, |a, b| Ok(a | b?))
    }
}

impl Equals {
    pub fn new(l: Expr, r: Expr) -> Self {
        Self(Box::new(l), Box::new(r))
    }

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        let mut l = String::new();
        let mut r = String::new();
        let lres = self.0.eval(&mut l, ctx)?;
        let rres = self.1.eval(&mut r, ctx)?;

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

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        let mut w = FakeWriter;
        if self.cond.eval(&mut w, ctx)? {
            self.success.eval(res, ctx)
        } else {
            self.failure.eval(res, ctx)
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

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        let mut w = String::new();
        if self.cond.eval(&mut w, ctx)? {
            res.write_str(&w)?;
            Ok(true)
        } else {
            self.other.eval(res, ctx)
        }
    }
}

impl Call {
    pub fn new(
        typ: Variable,
        file: Expr,
        define: HashMap<Variable, Expr>,
        undefine: Vec<Variable>,
    ) -> Self {
        Self {
            typ,
            file: Box::new(file),
            define,
            undefine,
        }
    }

    pub fn eval<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        match self.typ.0.as_str() {
            "exists" => self.exists(ctx),
            "include" => self.include(res, ctx),
            "make" => self.make(res, ctx),
            a => Err(Error::Msg(format!("Unknown function '{a}'").into())),
        }
    }

    pub fn exists(self, ctx: ExpandContext) -> Result<bool> {
        if !self.define.is_empty() || !self.undefine.is_empty() {
            return Err(Error::Msg(
                "Too many arguments to function '#include'".into(),
            ));
        }

        let mut file = String::new();
        self.file.eval(&mut file, ctx)?;
        let file = ctx.template_dir.join(file);
        Ok(file.exists())
    }

    pub fn include<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        if !self.define.is_empty() || !self.undefine.is_empty() {
            return Err(Error::Msg(
                "Too many arguments to function '#include'".into(),
            ));
        }

        let mut file = String::new();
        self.file.eval(&mut file, ctx)?;
        let file = ctx.template_dir.join(file);
        if !file.exists() {
            return Ok(false);
        }

        let mut file = BufReader::new(File::open(file)?);
        for c in file.chars() {
            res.write_char(c?)?;
        }

        Ok(true)
    }

    pub fn make<W>(self, res: &mut W, ctx: ExpandContext) -> Result<bool>
    where
        W: Write,
    {
        let mut file = String::new();
        self.file.eval(&mut file, ctx)?;
        let file = ctx.template_dir.join(file);
        if !file.exists() {
            return Ok(false);
        }

        let mut file = BufReader::new(File::open(file)?);
        if self.define.is_empty() && self.undefine.is_empty() {
            expand(ctx, &mut file.chars().map(|a| Ok(a?)), res)?;
            return Ok(true);
        }

        let mut vars: HashMap<Cow<_>, Cow<_>> = ctx
            .vars
            .iter()
            .map(|(k, v)| (k.as_ref().into(), v.as_ref().into()))
            .collect();

        for k in self.undefine {
            vars.remove(k.0.as_str());
        }

        for (k, v) in self.define {
            let mut value = String::new();
            v.eval(&mut value, ctx)?;
            vars.insert(k.0.into(), value.into());
        }

        expand(
            ExpandContext {
                vars: &vars,
                template_dir: ctx.template_dir,
            },
            &mut file.chars().map(|a| Ok(a?)),
            res,
        )?;

        Ok(true)
    }
}
