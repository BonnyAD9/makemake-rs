use thiserror::Error;

use crate::lexer::Token;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected character '{}'", .0)]
    LexerUnexpected(char),
    #[error("expected {}", .0)]
    LexerExpect(&'static str),
    #[error("unexpected token '{}'", .0)]
    ParserUnexpected(Token),
    #[error("expected {}", .0)]
    ParserExpected(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
}
