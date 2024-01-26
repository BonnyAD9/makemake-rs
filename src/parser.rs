use result::OptionResultExt;

use crate::{
    ast::{Condition, Expr, Literal, Variable},
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
        if !matches!(self.cur, Some(Token::CloseBracket)) {
            Err(Error::ParserExpected("'}'"))
        } else {
            Ok(res)
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        self.get_tok()?;

        let mut res = Expr::None;

        while let Some(t) = self.cur.take() {
            match t {
                Token::Question => return self.condition(res),
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

    fn next_tok(&mut self) -> Result<()> {
        self.cur = self.lexer.next().invert()?;
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
