use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArgError {
    #[error("Expected {} after option '{}'", .exp, .opt)]
    MissingAfter { exp: &'static str, opt: String },
    #[error(
        "Invalid option '{}', If you ment to load template, use the option \
        '--load' for templates which name starts with '-'", .0
    )]
    InvalidOption(String),
    #[error("{}", .0)]
    _OtherMsg(&'static str),
}

pub type ArgResult<T> = Result<T, ArgError>;

#[derive(Clone, Copy)]
pub enum PromptAnswer {
    Yes,
    No,
    Ask,
}

pub enum Action<'a> {
    None,
    Help,
    Create { name: &'a str, path: &'a str },
    Load { name: &'a str, path: &'a str },
    Remove(&'a str),
    List,
    Edit { name: &'a str, path: &'a str },
}

impl<'a> Action<'a> {
    pub const fn _is_set(&self) -> bool {
        !matches!(self, Action::None)
    }
}

pub struct Args<'a> {
    pub action: Action<'a>,
    pub vars: HashMap<String, String>,
    pub prompt_answer: PromptAnswer,
}

struct ArgIterator<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    inner: I,
    last: Option<&'a str>,
}

impl<'a, I> ArgIterator<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    pub fn new(inner: I) -> Self {
        Self { inner, last: None }
    }

    pub fn next(&mut self) -> Option<&'a str> {
        self.last = self.inner.next();
        self.last
    }

    pub fn expect(&mut self, msg: &'static str) -> ArgResult<&'a str> {
        if let Some(next) = self.inner.next() {
            self.last = Some(next);
            Ok(next)
        } else {
            let cur = self.last.unwrap_or("");
            Err(ArgError::MissingAfter {
                exp: msg,
                opt: cur.to_owned(),
            })
        }
    }
}

impl<'a> Args<'a> {
    pub fn parse<I>(args: I) -> ArgResult<Self>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut res = Self {
            action: Action::None,
            vars: HashMap::new(),
            prompt_answer: PromptAnswer::Ask,
        };

        let mut args = ArgIterator::new(args);

        while let Some(arg) = args.next() {
            match arg {
                "-h" | "--help" => res.action = Action::Help,
                "-c" | "--create" => {
                    res.action = Action::Create {
                        name: args.expect("new template name")?,
                        path: ".",
                    }
                }
                "-cf" | "--create-from" => {
                    res.action = Action::Create {
                        name: args.expect(
                            "new template name and new source directory",
                        )?,
                        path: args.expect("source directory")?,
                    }
                }
                "--load" => {
                    res.action = Action::Load {
                        name: args.expect("existing template name")?,
                        path: ".",
                    }
                }
                "-lt" | "--load-to" => {
                    res.action = Action::Load {
                        name: args.expect(
                            "existing template name and destination directory",
                        )?,
                        path: args.expect("destination directory")?,
                    }
                }
                "-l" | "--list" => res.action = Action::List,
                "-r" | "--remove" => {
                    res.action =
                        Action::Remove(args.expect("existing template name")?)
                }
                "-e" | "--edit" => {
                    res.action = Action::Edit {
                        name: args.expect("existing template name")?,
                        path: ".",
                    }
                }
                "-ei" | "--edit-in" => {
                    res.action = Action::Edit {
                        name: args.expect(
                            "existing template name and destination directory",
                        )?,
                        path: args.expect("destination directory")?,
                    }
                }
                "-p" | "--prompt-answer" => {
                    let answ = args.expect("'yes', 'no' or 'ask'")?;
                    res.prompt_answer = match answ.to_lowercase().as_str() {
                        "yes" => PromptAnswer::Yes,
                        "no" => PromptAnswer::No,
                        "ask" => PromptAnswer::Ask,
                        _ => {
                            return Err(ArgError::MissingAfter {
                                exp: "'yes', 'no' or 'ask'",
                                opt: arg.to_owned(),
                            })
                        }
                    }
                }
                "-py" => res.prompt_answer = PromptAnswer::Yes,
                "-pn" => res.prompt_answer = PromptAnswer::No,
                "-pa" => res.prompt_answer = PromptAnswer::Ask,
                arg if arg.starts_with("-D") => {
                    let arg = &arg[2..];
                    if let Some((name, value)) = arg.split_once("=") {
                        res.vars.insert(name.to_owned(), value.to_owned());
                    } else {
                        res.vars.insert(arg.to_owned(), " ".to_owned());
                    }
                }
                arg if arg.starts_with("-") => {
                    return Err(ArgError::InvalidOption(arg.to_owned()))
                }
                _ => {
                    res.action = Action::Load {
                        name: arg,
                        path: ".",
                    }
                }
            }
        }

        Ok(res)
    }
}
