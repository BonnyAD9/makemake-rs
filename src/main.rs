use args::{Action, Args, Yna};
use dirs::config_dir;
use err::Result;
use maker::{copy_dir, create_template, load_template};
use std::{
    borrow::Cow,
    env,
    fs::{read_dir, remove_dir_all},
    io::{stderr, stdin, stdout, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use termal::{eprintmcln, printmcln};

use crate::err::Error;

mod args;
mod ast;
mod err;
mod lexer;
mod maker;
mod parser;
mod writer;

fn main() -> ExitCode {
    match start() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintmcln!(stderr().is_terminal(), "{'r}error:{'_} {e}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let args = Args::parse(args.iter().skip(1).map(|a| a.as_str()))?;

    args.check_unused();

    // Do what the arguments specify
    match args.get_action() {
        Action::Create => create(args)?,
        Action::Load => load(args)?,
        Action::Remove => remove(args)?,
        Action::Edit => edit(args)?,
        Action::Help => help(args),
        Action::Version => version(args),
        Action::List => list()?,
    }

    Ok(())
}

/// Creates new template with the name `name` from the directory `src` in the
/// default template folder.
fn create(args: Args) -> Result<()> {
    let name = args.get_template()?;
    let src = args.get_directory();

    let tdir = get_template_dir(name)?;

    if Path::new(&tdir).exists() {
        if !prompt_yn(
            &format!(
                "Template with the name '{name}' already \
            exists.\nDo you want to overwrite it? [y/N]: "
            ),
            args.prompt_answer,
        )? {
            return Ok(());
        }
        remove_dir_all(&tdir)?;
    }

    create_template(src, tdir)
}

/// Loads template with the name `src` to the directory `dest`. `vars` can
/// add/override variables in the template config file.
fn load(args: Args) -> Result<()> {
    let name = args.get_template()?;
    let dest = args.get_directory();

    let template = get_template_dir(name)?;
    if !template.exists() {
        return Err(Error::Msg(
            format!("There is no existing template '{name}'").into(),
        ));
    }

    // true if the directory exists and isn't empty
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some()
        && !prompt_yn(
            &format!(
                "the directory {dest} is not empty.\n\
            Do you want to load the template anyway? [y/N]: "
            ),
            args.prompt_answer,
        )?
    {
        return Ok(());
    }

    load_template(template, dest, args.vars)
}

/// Deletes template with the name `name`
fn remove(args: Args) -> Result<()> {
    remove_dir_all(get_template_dir(args.get_template()?)?)?;
    Ok(())
}

/// Copies the template with the name `name` to the directory `dest`.
fn edit(args: Args) -> Result<()> {
    let name = args.get_template()?;
    let dest = args.get_directory();

    let template = get_template_dir(name)?;
    if !template.exists() {
        return Err(Error::Msg(
            format!("There is no existing template '{name}'").into(),
        ));
    }

    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some()
        && !prompt_yn(
            &format!(
                "the directory {dest} is not empty.\n\
            Do you want to load the template source anyway? [y/N]: "
            ),
            args.prompt_answer,
        )?
    {
        return Ok(());
    }
    copy_dir(template, dest)
}

/// Prints all the template name to the stdout.
fn list() -> Result<()> {
    for f in read_dir(get_template_dir("")?)? {
        println!("{}", f?.file_name().to_string_lossy());
    }
    Ok(())
}

/// Writes the string `prompt` to the stdout and waits for the user to
/// enter either 'y' or 'n'. If the user enters something other than
/// 'y' or 'n' the function reurns Err. If the user enters 'y' the
/// function returns Ok(Some(())) otherwise returns Ok(None)
fn prompt_yn(prompt: &str, answ: Yna) -> Result<bool> {
    match answ {
        Yna::Auto => {}
        Yna::No => return Ok(false),
        Yna::Yes => return Ok(true),
    }
    print!("{prompt}");
    _ = stdout().flush();
    let mut conf = String::new();
    stdin().read_line(&mut conf)?;
    let conf = conf.trim();

    match conf {
        "y" | "Y" => Ok(true),
        "n" | "N" | "" => Ok(false),
        _ => Err(Error::Msg(format!("Invalid option {conf}").into())),
    }
}

/// Gets the directory in which the template with the name `name` is stored.
fn get_template_dir(name: &str) -> Result<PathBuf> {
    let mut config =
        config_dir().ok_or(Error::Msg("Can't get config directory".into()))?;
    config.push("makemake/templates");
    config.push(name);
    Ok(config)
}

fn version(args: Args) {
    let v = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    let signature: Cow<str> = if args.use_color() {
        termal::gradient("BonnyAD9", (250, 50, 170), (180, 50, 240)).into()
    } else {
        "BonnyAD9".into()
    };

    let exe = std::env::current_exe();
    let exe = exe
        .as_ref()
        .map(|e| e.to_string_lossy())
        .unwrap_or("unknown".into());

    printmcln!(
        args.use_color(),
        "makemake v{v}
Author: {signature}{'_}
Exe path: {exe}
"
    )
}

/// Prints colorful help to the stdout.
fn help(args: Args) {
    let v: Option<&str> = option_env!("CARGO_PKG_VERSION");
    let signature: Cow<str> = if args.use_color() {
        termal::gradient("BonnyAD9", (250, 50, 170), (180, 50, 240)).into()
    } else {
        "BonnyAD9".into()
    };

    printmcln!(
        args.use_color(),
        "Welcome in {'g i}makemake{'_} by {signature}{'_}
Version {}

{'g}Usage:
  {'w}makemake <template name> {'gr}[options]{'_}
    Behaves according to the options, by default loads template.

  {'w}makemake {'gr}[options]{'_}
    Bahaves according to the options, with no options shows this help.

{'g}Options:
  {'y}-h  -?  --help{'_}
    Shows this help.

  {'y}-c  --create{'_}
    Creates new template.

  {'y}-r  --remove{'_}
    Removes the template.

  {'y}-l  --list{'_}
    Lists all the template names.

  {'y}-D{'w}<variable name>{'gr}[=value]{'_}
    Defines/redefines a variable.

  {'y}-e  --edit {'w}<template name>{'_}
    Loads template source to this directory. If the directory is destination
    directory and it doesn't exist, it will be created.

  {'y}-p  --prompt {'w}<yes | no | ask>{'_}
    Sets the default answer when prompting. 'yes' will always answer with 'y',
    'no' will always answer with 'n', 'ask' is default - always ask. Can be
    overriden.

  {'y}-py{'_}, {'y}-pn{'_}, {'y}-pa{'_}
    Same as '-p yes', '-p no' and '-p ask' respectively.

  {'y}-d  --directory {'w}<path to directory>{'_}
    Sets the relevant directory path. This is cwd by default.

  {'y}-t  --template {'w}<template name>{'_}
    Sets the template, this can be used for templates which name starts with
    '-'.

  {'y}--load {'w}<template name>{'_}
    Loads the given template.

  {'y}-cf  --create-from {'w}<template name> <template surce directory>{'_}
    Creates new template from the directory with the name. (Equivalent to
    '-c -t <template name> -d <template source directory>'.)

  {'y}-lt  --load-to {'w}<template name> <destination directory>{'_}
    Loads the given template into the destination directory (will be created
    if it doesn't exist). (Equivalent to
    '--load <temlate name> -d <destination directory>'.)

  {'y}-ei --edit-in {'w}<template name> <directory>{'_}
    Loads template source to the given direcotry. (Equivalent to
    '-e -t <template name> -d <template source directory>'.)

  {'y}--color  --colour {'w}<auto | always | never>
  {'y}--color  --colour{'w}=<auto | always | never>{'_}
    Determines whether to use color when printing.

Ehen option can be overriden, it means that it can be specified multiple
times, and the last occurence takes effect.

See {'w bold}makemake(7){'_} for description of the template format.
",
        v.unwrap_or("unknown")
    );
}
