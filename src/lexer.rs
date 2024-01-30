use std::fmt::{Display, Write};

use crate::err::{Error, Result};
use result::prelude::*;

#[derive(Debug)]
pub enum Token {
    CloseBracket,
    Question,
    Colon,
    OpenParen,
    CloseParen,
    Equals,
    Ident(String),
    Literal(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CloseBracket => f.write_char('}'),
            Self::Question => f.write_char('?'),
            Self::Colon => f.write_char(':'),
            Self::OpenParen => f.write_char('('),
            Self::CloseParen => f.write_char(')'),
            Self::Equals => f.write_str("=="),
            Self::Ident(i) => f.write_str(i),
            Self::Literal(l) => f.write_str(l),
        }
    }
}

pub struct Lexer<'a, I>
where
    I: Iterator<Item = Result<char>>,
{
    cur: Option<char>,
    data: &'a mut I,
}

impl<'a, I> Iterator for Lexer<'a, I>
where
    I: Iterator<Item = Result<char>>,
{
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_next().invert()
    }
}

impl<'a, I> From<&'a mut I> for Lexer<'a, I>
where
    I: Iterator<Item = Result<char>>,
{
    fn from(value: &'a mut I) -> Self {
        Self {
            cur: None,
            data: value,
        }
    }
}

impl<'a, I> Lexer<'a, I>
where
    I: Iterator<Item = Result<char>>,
{
    fn inner_next(&mut self) -> Result<Option<Token>> {
        if self.cur.is_none() {
            self.next_chr()?;
        }

        while let Some(c) = self.cur {
            if !c.is_whitespace() {
                break;
            }
            self.next_chr()?;
        }

        match self.cur {
            Some('}') => {
                self.cur = None;
                Ok(Some(Token::CloseBracket))
            }
            Some('?') => {
                self.cur = None;
                Ok(Some(Token::Question))
            }
            Some(':') => {
                self.cur = None;
                Ok(Some(Token::Colon))
            }
            Some('(') => {
                self.cur = None;
                Ok(Some(Token::OpenParen))
            }
            Some(')') => {
                self.cur = None;
                Ok(Some(Token::CloseParen))
            }
            Some('=') => {
                self.next_chr()?;
                if !matches!(self.cur, Some('=')) {
                    Err(Error::LexerExpect("'='"))
                } else {
                    self.cur = None;
                    Ok(Some(Token::Equals))
                }
            }
            Some('\'') => self.read_literal(),
            Some(a) if a.is_alphabetic() || a == '_' => self.read_ident(),
            None => Ok(None),
            Some(c) => Err(Error::LexerUnexpected(c)),
        }
    }

    fn read_ident(&mut self) -> Result<Option<Token>> {
        let mut ident = String::new();

        // should be always true
        if let Some(c) = self.cur {
            ident.push(c);
        }

        while let Some(c) = self.next_chr()? {
            if !c.is_alphanumeric() && c != '_' {
                break;
            }
            ident.push(c);
        }

        Ok(Some(Token::Ident(ident)))
    }

    fn read_literal(&mut self) -> Result<Option<Token>> {
        let mut lit = String::new();

        let mut success = false;
        while let Some(c) = self.next_chr()? {
            match c {
                '\'' => {
                    success = true;
                    self.next_chr()?;
                    break;
                }
                '\\' => lit.push(self.escape()?),
                _ => lit.push(c),
            }
        }

        if !success {
            return Err(Error::LexerExpect("`'` to close the literal"));
        }

        Ok(Some(Token::Literal(lit)))
    }

    fn escape(&mut self) -> Result<char> {
        if let Some(c) = self.next_chr()? {
            match c {
                'n' => Ok('\n'),
                'r' => Ok('\r'),
                't' => Ok('\t'),
                '\\' => Ok('\\'),
                '\'' => Ok('\''),
                c => Ok(c),
            }
        } else {
            Err(Error::LexerExpect("escape sequence"))
        }
    }

    fn next_chr(&mut self) -> Result<Option<char>> {
        self.cur = self.data.next().invert()?;
        Ok(self.cur)
    }
}
