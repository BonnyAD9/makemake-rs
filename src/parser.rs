use std::collections::HashMap;

use crate::{
    ast::{Call, Condition, Equals, Expr, Literal, NullCheck, Variable},
    err::{Error, Result},
    lexer::{Lexer, Token},
};

pub struct Parser<I>
where
    I: Iterator<Item = Result<Token>>,
{
    lexer: I,
    cur: Option<Token>,
}

pub fn parse<I>(data: &mut I) -> Result<Expr>
where
    I: Iterator<Item = Result<char>>,
{
    let lexer: Lexer<I> = data.into();
    let mut parser = Parser::new(lexer);
    parser.parse()
}

impl<I> Parser<I>
where
    I: Iterator<Item = Result<Token>>,
{
    pub fn new(lexer: I) -> Self {
        Self { lexer, cur: None }
    }

    pub fn parse(&mut self) -> Result<Expr> {
        let res = self.expr()?;

        self.get_tok()?;
        match self.cur {
            None | Some(Token::CloseBracket) => Ok(res),
            _ => Err(Error::ParserExpected("'}'")),
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        self.get_tok()?;

        let mut res = Expr::None;

        while let Some(t) = self.cur.take() {
            match t {
                Token::Question => return self.condition(res),
                Token::NullCheck => return self.null_check(res),
                Token::OpenParen => res.concat(self.paren()?),
                Token::Pound => res.concat(self.call()?),
                Token::Equals => res = self.equals(res)?,
                Token::Ident(i) => res.concat(Variable::new(i).into()),
                Token::Literal(l) => res.concat(Literal::new(l).into()),
                _ => {
                    self.cur = Some(t);
                    break;
                }
            }
            self.get_tok()?;
        }

        Ok(res)
    }

    fn paren(&mut self) -> Result<Expr> {
        let res = self.expr()?;
        if !matches!(self.cur, Some(Token::CloseParen)) {
            Err(Error::ParserExpected("')'"))
        } else {
            self.cur.take();
            Ok(res)
        }
    }

    fn concat(&mut self) -> Result<Expr> {
        self.get_tok()?;

        let mut res = Expr::None;

        while let Some(t) = self.cur.take() {
            match t {
                Token::OpenParen => res.concat(self.paren()?),
                Token::Ident(i) => res.concat(Variable::new(i).into()),
                Token::Literal(l) => res.concat(Literal::new(l).into()),
                _ => {
                    self.cur = Some(t);
                    break;
                }
            }
            self.next_tok()?;
        }

        Ok(res)
    }

    fn equals(&mut self, l: Expr) -> Result<Expr> {
        let r = self.concat()?;
        Ok(Equals::new(l, r).into())
    }

    fn condition(&mut self, cond: Expr) -> Result<Expr> {
        let success = self.expr()?;

        self.get_tok()?;
        if !matches!(self.cur, Some(Token::Colon)) {
            return Err(Error::ParserExpected("':'"));
        }

        self.next_tok()?;
        let failure = self.expr()?;

        Ok(Condition::new(cond, success, failure).into())
    }

    fn null_check(&mut self, cond: Expr) -> Result<Expr> {
        let other = self.expr()?;

        Ok(NullCheck::new(cond, other).into())
    }

    fn call(&mut self) -> Result<Expr> {
        self.get_tok()?;

        let Some(Token::Ident(ident)) = self.cur.take() else {
            return Err(Error::ParserExpected("identifier after '#'"));
        };

        self.get_tok()?;
        if !matches!(self.cur, Some(Token::OpenParen)) {
            return Err(Error::ParserExpected("'('"));
        }
        self.next_tok()?;

        let file = self.expr()?;

        let mut define = HashMap::new();
        let mut undefine = Vec::new();

        while matches!(self.cur, Some(Token::Comma)) {
            self.next_tok()?;
            if matches!(self.cur, Some(Token::CloseParen)) {
                break;
            }

            let mut undef = false;
            if matches!(self.cur, Some(Token::Minus)) {
                undef = true;
                self.next_tok()?;
            }

            let Some(Token::Ident(ident)) = self.cur.take() else {
                return Err(Error::ParserExpected("Identifer in arguments."));
            };

            let mut value = Expr::None;
            self.get_tok()?;
            if matches!(self.cur, Some(Token::Assign)) {
                self.next_tok()?;
                value = self.expr()?;
            }
            self.get_tok()?;

            if undef {
                undefine.push(Variable::new(ident));
            } else {
                define.insert(Variable::new(ident), value);
            }
        }

        self.get_tok()?;
        if !matches!(self.cur, Some(Token::CloseParen)) {
            return Err(Error::ParserExpected("')'"));
        }
        self.next_tok()?;

        Ok(Call::new(Variable::new(ident), file, define, undefine).into())
    }

    fn next_tok(&mut self) -> Result<()> {
        self.cur = self.lexer.next().transpose()?;
        Ok(())
    }

    fn get_tok(&mut self) -> Result<()> {
        if self.cur.is_none() {
            self.next_tok()
        } else {
            Ok(())
        }
    }
}
