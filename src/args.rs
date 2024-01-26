use std::collections::HashMap;

use termal::eprintcln;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArgError {
    #[error("Expected {} after option '{}'", .exp, .opt)]
    MissingAfter { exp: &'static str, opt: String },
    #[error(
        "Invalid option '{}', If you ment it as template name, use '-t' for \
        templates which name starts with '-'", .0
    )]
    InvalidOption(String),
    #[error("In the arguments the template is set multiple times.")]
    TooManyTemplates,
    #[error("In the arguments the working path is set multiple times.")]
    TooManyPaths,
    #[error("In the arguments the action is set multiple times.")]
    TooManyActions,
    #[error("Missing template name for the given action.")]
    MissingTemplate,
}

pub type ArgResult<T> = Result<T, ArgError>;

#[derive(Clone, Copy)]
pub enum PromptAnswer {
    Yes,
    No,
    Ask,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Help,
    Create,
    Load,
    Remove,
    List,
    Edit,
}

pub struct Args<'a> {
    pub cnt: usize,
    pub template: Option<&'a str>,
    pub directory: Option<&'a str>,
    pub action: Option<Action>,
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
            cnt: 0,
            template: None,
            directory: None,
            action: None,
            vars: HashMap::new(),
            prompt_answer: PromptAnswer::Ask,
        };

        let mut args = ArgIterator::new(args);

        while let Some(arg) = args.next() {
            res.cnt += 1;
            match arg {
                "-h" | "--help" | "-?" => res.set_action(Action::Help)?,
                "-c" | "--create" => res.set_action(Action::Create)?,
                "-t" | "--template" => {
                    res.set_template(args.expect("template name")?)?
                }
                "-d" | "--directory" => {
                    res.set_path(args.expect("path to a directory")?)?
                }
                "-cf" | "--create-from" => res.set_action_template_path(
                    Action::Create,
                    args.expect("new template name and new source directory")?,
                    args.expect("source directory")?,
                )?,
                "--load" => res.set_action_template(
                    Action::Load,
                    args.expect("existing template name")?,
                )?,
                "-lt" | "--load-to" => res.set_action_template_path(
                    Action::Load,
                    args.expect(
                        "existing template name and destination directory",
                    )?,
                    args.expect("destination directory")?,
                )?,
                "-l" | "--list" => res.set_action(Action::List)?,
                "-r" | "--remove" => res.set_action(Action::Remove)?,
                "-e" | "--edit" => res.set_action(Action::Edit)?,
                "-ei" | "--edit-in" => res.set_action_template_path(
                    Action::Edit,
                    args.expect(
                        "existing template name and destination directory",
                    )?,
                    args.expect("destination directory")?,
                )?,
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
                        res.vars.insert(arg.to_owned(), String::new());
                    }
                }
                arg if arg.starts_with("-") => {
                    return Err(ArgError::InvalidOption(arg.to_owned()))
                }
                _ => res.set_template(arg)?,
            } // match
        } // while

        Ok(res)
    } // fn parse

    pub fn check_unused(&self) {
        match self.get_action() {
            Action::Help => {
                self.unused_template();
                self.unused_directory();
                self.unused_vars();
            },
            Action::Create => {
                self.unused_vars();
            }
            Action::Load => {}
            Action::Remove => {
                self.unused_directory();
                self.unused_vars();
            },
            Action::List => {
                self.unused_template();
                self.unused_directory();
                self.unused_vars();
            },
            Action::Edit => {
                self.unused_vars();
            }
        }
    }

    fn unused_template(&self) {
        if let Some(t) = self.template {
            eprintcln!("{'m}warning:{'_} unused template argument '{t}'");
        }
    }

    fn unused_directory(&self) {
        if let Some(d) = self.directory {
            eprintcln!("{'m}warning:{'_} unused directory argument '{d}'");
        }
    }

    fn unused_vars(&self) {
        if !self.vars.is_empty() {
            eprintcln!("{'m}warning:{'_} variables are set but unused.");
        }
    }

    pub fn get_directory(&self) -> &'a str {
        self.directory.unwrap_or(".")
    }

    pub fn get_template(&self) -> ArgResult<&'a str> {
        self.template.ok_or(ArgError::MissingTemplate)
    }

    pub fn get_action(&self) -> Action {
        if self.cnt == 0 {
            Action::Help
        } else {
            self.action.unwrap_or(Action::Load)
        }
    }

    fn set_template(&mut self, template: &'a str) -> ArgResult<()> {
        if self.template.is_some() {
            Err(ArgError::TooManyTemplates)
        } else {
            self.template = Some(template);
            Ok(())
        }
    }

    fn set_path(&mut self, path: &'a str) -> ArgResult<()> {
        if self.directory.is_some() {
            Err(ArgError::TooManyPaths)
        } else {
            self.directory = Some(path);
            Ok(())
        }
    }

    fn set_action(&mut self, action: Action) -> ArgResult<()> {
        if self.action.is_some() {
            Err(ArgError::TooManyActions)
        } else {
            self.action = Some(action);
            Ok(())
        }
    }

    fn set_action_template(
        &mut self,
        action: Action,
        template: &'a str,
    ) -> ArgResult<()> {
        self.set_action(action)?;
        self.set_template(template)
    }

    fn set_action_template_path(
        &mut self,
        action: Action,
        template: &'a str,
        path: &'a str,
    ) -> ArgResult<()> {
        self.set_action(action)?;
        self.set_template(template)?;
        self.set_path(path)
    }
} // impl Args<'a>
