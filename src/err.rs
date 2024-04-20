use std::{borrow::Cow, path::StripPrefixError};

use thiserror::Error;

use crate::args::ArgError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected character '{}'", .0)]
    LexerUnexpected(char),
    #[error("expected {}", .0)]
    LexerExpect(&'static str),
    #[error("expected {}", .0)]
    ParserExpected(&'static str),
    #[error("{}", .0)]
    Msg(Cow<'static, str>),
    #[error("Command {} failed with stderr:\n{}", .cmd, .stderr)]
    CommandUnsuccessful { cmd: String, stderr: String },
    #[error(transparent)]
    Arg(#[from] ArgError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    StripPrefix(#[from] StripPrefixError),
    #[error(transparent)]
    ShellParseError(#[from] shell_words::ParseError),
}
