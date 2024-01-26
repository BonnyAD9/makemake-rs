use std::borrow::Cow;

use thiserror::Error;

use crate::args::ArgError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected character '{}'", .0)]
    LexerUnexpected(char),
    #[error("expected {}", .0)]
    LexerExpect(&'static str),
    //#[error("unexpected token '{}'", .0)]
    //ParserUnexpected(Token),
    #[error("expected {}", .0)]
    ParserExpected(&'static str),
    #[error("{}", .0)]
    Unsupported(&'static str),
    #[error("{}", .0)]
    Msg(Cow<'static, str>),
    #[error(transparent)]
    Arg(#[from] ArgError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}
