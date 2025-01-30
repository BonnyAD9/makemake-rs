use std::{
    borrow::Cow,
    collections::HashMap,
    io::{stdout, IsTerminal},
};

use pareg::{has_any_key, key_mval_arg, FromArg, Pareg, Result};

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

#[derive(Clone, Debug, Default)]
pub enum Action {
    #[default]
    Help,
    Version,
    Create,
    Alias(String),
    Config,
    Load,
    Remove,
    List,
    Edit,
}

pub struct Args {
    pub use_color: Yna,
    pub template: String,
    pub directory: Cow<'static, str>,
    pub action: Action,
    pub vars: HashMap<Cow<'static, str>, Cow<'static, str>>,
    pub prompt_answer: Yna,
}

impl Args {
    pub fn parse(mut args: Pareg) -> Result<Self> {
        let mut use_color = Yna::Auto;
        let mut template = None;
        let mut directory: Cow<'static, str> = ".".into();
        let mut vars = HashMap::new();
        let mut prompt_answer = Yna::Auto;
        let mut action = None;

        // TODO: use try_set after pareg update

        while let Some(arg) = args.next() {
            match arg {
                "-h" | "--help" | "-?" => {
                    set_action(&mut action, Action::Help, &args)?
                }
                "--version" => {
                    set_action(&mut action, Action::Version, &args)?
                }
                "-c" | "--create" => {
                    set_action(&mut action, Action::Create, &args)?
                }
                "-a" | "--alias" => {
                    set_action(
                        &mut action,
                        Action::Alias(args.next_arg()?),
                        &args,
                    )?;
                }
                "-C" | "--config" | "--configure" => {
                    set_action(&mut action, Action::Config, &args)?
                }
                "-t" | "--template" => template = Some(args.next_arg()?),
                "-d" | "--directory" => {
                    directory = args.next_arg::<String>()?.into()
                }
                "-cf" | "--create-from" => {
                    set_action(&mut action, Action::Create, &args)?;
                    template = Some(args.next_arg()?);
                    directory = args.next_arg::<String>()?.into();
                }
                "--load" => {
                    set_action(&mut action, Action::Load, &args)?;
                    template = Some(args.next_arg()?);
                }
                "-lt" | "--load-to" => {
                    set_action(&mut action, Action::Load, &args)?;
                    template = Some(args.next_arg()?);
                    directory = args.next_arg::<String>()?.into();
                }
                "-l" | "--list" => {
                    set_action(&mut action, Action::List, &args)?
                }
                "-r" | "--remove" => {
                    set_action(&mut action, Action::Remove, &args)?
                }
                "-e" | "--edit" => {
                    set_action(&mut action, Action::Edit, &args)?
                }
                "-ei" | "--edit-in" => {
                    set_action(&mut action, Action::Edit, &args)?;
                    template = Some(args.next_arg()?);
                    directory = args.next_arg::<String>()?.into();
                }
                "-p" | "--prompt-answer" => {
                    prompt_answer = args.next_arg()?;
                }
                "-py" => prompt_answer = Yna::Yes,
                "-pn" => prompt_answer = Yna::No,
                "-pa" => prompt_answer = Yna::Auto,
                v if has_any_key!(v, '=', "--color", "--colour") => {
                    use_color = args.cur_val_or_next('=')?;
                }
                arg if arg.starts_with("-D") => {
                    let arg = &arg[2..];
                    let (k, v) = key_mval_arg::<String, String>(arg, '=')?;
                    vars.insert(
                        k.into(),
                        v.map(|v| v.into()).unwrap_or("".into()),
                    );
                }
                arg if arg.starts_with('-') => {
                    return args
                        .err_unknown_argument()
                        .hint(
                            "If this was ment to be template name, \
                        use `-t` to explicitly set the template.",
                        )
                        .err();
                }
                _ => template = Some(args.cur_arg()?),
            } // match
        } // while

        if use_color == Yna::Auto {
            use_color = if stdout().is_terminal() {
                Yna::Yes
            } else {
                Yna::No
            };
        }

        let action = action
            .or(template.is_some().then_some(Action::Load))
            .unwrap_or_default();
        if action.needs_template() && template.is_none() {
            Err(args
                .err_no_more_arguments()
                .main_msg("Expected template name.")
                .inline_msg("Add template name.")
                .hint(
                    "Use `-t <template>` to set template name that starts \
                    with `-`.",
                ))
        } else {
            Ok(Self {
                use_color,
                template: template.unwrap_or_default(),
                directory,
                action,
                vars,
                prompt_answer,
            })
        }
    } // fn parse

    pub fn use_color(&self) -> bool {
        self.use_color == Yna::Yes
    }
} // impl Args<'a>

impl Action {
    pub fn needs_template(&self) -> bool {
        use self::Action::*;
        matches!(self, Create | Alias(_) | Load | Remove | Edit)
    }
}

fn assert_no_action(act: &Option<Action>, args: &Pareg) -> Result<()> {
    if act.is_some() {
        Err(args
            .err_invalid()
            .inline_msg("This is second action to do.")
            .main_msg("Cannot set do more actions."))
    } else {
        Ok(())
    }
}

fn set_action(
    act: &mut Option<Action>,
    action: Action,
    args: &Pareg,
) -> Result<()> {
    assert_no_action(act, args)?;
    *act = Some(action);
    Ok(())
}
