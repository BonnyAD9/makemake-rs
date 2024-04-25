use std::{
    borrow::Cow,
    collections::HashMap,
    io::{stdout, IsTerminal},
};

use pareg::{proc::FromArg, ArgIterator, ByRef};
use termal::eprintmcln;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArgError {
    #[error(transparent)]
    Pareg(pareg::ArgError<'static>),
    #[error("In the arguments the template is set multiple times.")]
    TooManyTemplates,
    #[error("In the arguments the working path is set multiple times.")]
    TooManyPaths,
    #[error("In the arguments the action is set multiple times.")]
    TooManyActions,
    #[error("Missing template name for the given action.")]
    MissingTemplate,
    #[error("Missing alias name for the given action.")]
    MissingAlias,
}

impl<'a> From<pareg::ArgError<'a>> for ArgError {
    fn from(value: pareg::ArgError<'a>) -> Self {
        Self::Pareg(value.into_owned())
    }
}

pub type ArgResult<T> = Result<T, ArgError>;

/// Yes/No/Auto
#[derive(FromArg, Clone, Copy, PartialEq, Eq)]
pub enum Yna {
    #[arg("always")]
    Yes,
    #[arg("never")]
    No,
    #[arg("ask")]
    Auto,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Help,
    Version,
    Create,
    Alias,
    Load,
    Remove,
    List,
    Edit,
}

pub struct Args<'a> {
    pub use_color: Yna,
    pub template: Option<&'a str>,
    pub alias: Option<&'a str>,
    pub directory: Option<&'a str>,
    pub action: Option<Action>,
    pub vars: HashMap<Cow<'a, str>, Cow<'a, str>>,
    pub prompt_answer: Yna,
}

impl<'a> Args<'a> {
    pub fn parse<I>(mut args: ArgIterator<'a, I>) -> ArgResult<Self>
    where
        I: Iterator,
        I::Item: ByRef<&'a str>,
    {
        let mut res = Self {
            use_color: Yna::Auto,
            template: None,
            alias: None,
            directory: None,
            action: None,
            vars: HashMap::new(),
            prompt_answer: Yna::Auto,
        };

        while let Some(arg) = args.next() {
            match arg {
                "-h" | "--help" | "-?" => res.set_action(Action::Help)?,
                "--version" => res.set_action(Action::Version)?,
                "-c" | "--create" => res.set_action(Action::Create)?,
                "-a" | "--alias" => res.set_alias(args.next_arg()?)?,
                "-t" | "--template" => res.set_template(args.next_arg()?)?,
                "-d" | "--directory" => res.set_path(args.next_arg()?)?,
                "-cf" | "--create-from" => res.set_action_template_path(
                    Action::Create,
                    args.next_arg()?,
                    args.next_arg()?,
                )?,
                "--load" => {
                    res.set_action_template(Action::Load, args.next_arg()?)?
                }
                "-lt" | "--load-to" => res.set_action_template_path(
                    Action::Load,
                    args.next_arg()?,
                    args.next_arg()?,
                )?,
                "-l" | "--list" => res.set_action(Action::List)?,
                "-r" | "--remove" => res.set_action(Action::Remove)?,
                "-e" | "--edit" => res.set_action(Action::Edit)?,
                "-ei" | "--edit-in" => res.set_action_template_path(
                    Action::Edit,
                    args.next_arg()?,
                    args.next_arg()?,
                )?,
                "-p" | "--prompt-answer" => {
                    res.prompt_answer = args.next_arg()?;
                }
                "-py" => res.prompt_answer = Yna::Yes,
                "-pn" => res.prompt_answer = Yna::No,
                "-pa" => res.prompt_answer = Yna::Auto,
                "--color" | "--colour" => {
                    res.use_color = args.next_arg()?;
                }
                arg if arg.starts_with("--color=")
                    || arg.starts_with("--colour=") =>
                {
                    res.use_color = args.cur_key_val::<&str, _>('=')?.1;
                }
                arg if arg.starts_with("-D") => {
                    let arg = &arg[2..];
                    if let Some((name, value)) = arg.split_once('=') {
                        res.vars.insert(name.into(), value.into());
                    } else {
                        res.vars.insert(arg.into(), "".into());
                    }
                }
                arg if arg.starts_with('-') => {
                    return Err(pareg::ArgError::UnknownArgument(arg.into()))?
                }
                _ => res.set_template(arg)?,
            } // match
        } // while

        if res.use_color == Yna::Auto {
            res.use_color = if stdout().is_terminal() {
                Yna::Yes
            } else {
                Yna::No
            };
        }

        Ok(res)
    } // fn parse

    pub fn check_unused(&self) {
        match self.get_action() {
            Action::Help | Action::Version => {
                self.unused_template();
                self.unused_directory();
                self.unused_vars();
            }
            Action::Create => {
                self.unused_vars();
            }
            Action::Alias => {
                self.unused_directory();
            }
            Action::Load => {}
            Action::Remove => {
                self.unused_directory();
                self.unused_vars();
            }
            Action::List => {
                self.unused_template();
                self.unused_directory();
                self.unused_vars();
            }
            Action::Edit => {
                self.unused_vars();
            }
        }
    }

    fn unused_template(&self) {
        if let Some(t) = self.template {
            eprintmcln!(
                self.use_color(),
                "{'m}warning:{'_} unused template argument '{t}'"
            );
        }
    }

    fn unused_directory(&self) {
        if let Some(d) = self.directory {
            eprintmcln!(
                self.use_color(),
                "{'m}warning:{'_} unused directory argument '{d}'"
            );
        }
    }

    fn unused_vars(&self) {
        if !self.vars.is_empty() {
            eprintmcln!(
                self.use_color(),
                "{'m}warning:{'_} variables are set but unused."
            );
        }
    }

    pub fn use_color(&self) -> bool {
        self.use_color == Yna::Yes
    }

    pub fn get_directory(&self) -> &'a str {
        self.directory.unwrap_or(".")
    }

    pub fn get_template(&self) -> ArgResult<&'a str> {
        self.template.ok_or(ArgError::MissingTemplate)
    }

    pub fn get_alias(&self) -> ArgResult<&'a str> {
        self.alias.ok_or(ArgError::MissingAlias)
    }

    pub fn get_action(&self) -> Action {
        if self.action.is_none() && self.template.is_none() {
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

    fn set_alias(&mut self, alias: &'a str) -> ArgResult<()> {
        self.set_action(Action::Alias)?;
        self.alias = Some(alias);
        Ok(())
    }
} // impl Args<'a>
