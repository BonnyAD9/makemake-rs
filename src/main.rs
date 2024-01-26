use args::{Action, Args, PromptAnswer};
use dirs::config_dir;
use err::Result;
use maker::{copy_dir, create_template, load_template};
use std::{
    env,
    fs::{read_dir, remove_dir_all},
    io::{stdin, stdout, Write},
    path::Path,
};
use termal::printcln;

use crate::err::Error;

mod args;
mod ast;
mod err;
mod lexer;
mod maker;
mod parser;
mod writer;

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let args = Args::parse(args.iter().skip(1).map(|a| a.as_str()))?;

    // Do what the arguments specify
    match &args.action {
        Action::Create { name, path } => create(name, path, args)?,
        Action::Load { name, path } => load(name, path, args)?,
        Action::Remove(n) => remove(n)?,
        Action::Edit { name, path } => edit(name, path, args)?,
        Action::Help | Action::None => help(),
        Action::List => list()?,
    }

    Ok(())
}

/// Creates new template with the name `name` from the directory `src` in the
/// default template folder.
fn create(name: &str, src: &str, args: Args) -> Result<()> {
    let tdir = get_template_dir(name)?;

    if Path::new(&tdir).exists() {
        if prompt_yn(
            &format!(
                "Template with the name '{name}' already \
            exists.\nDo you want to overwrite it? [y/N]: "
            ),
            args.prompt_answer,
        )?
        .is_none()
        {
            return Ok(());
        }
        remove_dir_all(&tdir)?;
    }

    create_template(src, &tdir)
}

/// Loads template with the name `src` to the directory `dest`. `vars` can
/// add/override variables in the template config file.
fn load(name: &str, dest: &str, args: Args) -> Result<()> {
    // true if the directory exists and isn't empty
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some() {
        if prompt_yn(
            &format!(
                "the directory {dest} is not empty.\n\
            Do you want to load the template anyway? [y/N]: "
            ),
            args.prompt_answer,
        )?
        .is_none()
        {
            return Ok(());
        }
    }

    load_template(&get_template_dir(name)?, dest, args.vars)
}

/// Deletes template with the name `name`
fn remove(name: &str) -> Result<()> {
    remove_dir_all(get_template_dir(name)?)?;
    Ok(())
}

/// Copies the template with the name `name` to the directory `dest`.
fn edit(name: &str, dest: &str, args: Args) -> Result<()> {
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some() {
        if prompt_yn(
            &format!(
                "the directory {dest} is not empty.\n\
            Do you want to load the template source anyway? [y/N]: "
            ),
            args.prompt_answer,
        )?
        .is_none()
        {
            return Ok(());
        }
    }
    copy_dir(get_template_dir(name)?.as_str(), dest)
}

/// Prints all the template name to the stdout.
fn list() -> Result<()> {
    for f in read_dir(get_template_dir("")?)? {
        println!(
            "{}",
            f?.file_name().to_string_lossy()
        );
    }
    Ok(())
}

/// Writes the string `prompt` to the stdout and waits for the user to
/// enter either 'y' or 'n'. If the user enters something other than
/// 'y' or 'n' the function reurns Err. If the user enters 'y' the
/// function returns Ok(Some(())) otherwise returns Ok(None)
fn prompt_yn(prompt: &str, answ: PromptAnswer) -> Result<Option<()>> {
    match answ {
        PromptAnswer::Ask => {}
        PromptAnswer::No => return Ok(None),
        PromptAnswer::Yes => return Ok(Some(())),
    }
    print!("{prompt}");
    _ = stdout().flush();
    let mut conf = String::new();
    stdin().read_line(&mut conf)?;
    let conf = conf.trim();

    return match conf {
        "y" | "Y" => Ok(Some(())),
        "n" | "N" | "" => Ok(None),
        _ => Err(Error::Msg(format!("Invalid option {conf}").into())),
    };
}

/// Gets the directory in which the template with the name `name` is stored.
fn get_template_dir(name: &str) -> Result<String> {
    let config =
        config_dir().ok_or(Error::Msg("Can't get config directory".into()))?;

    Ok(config
        .to_str()
        .ok_or(Error::Msg("Invalid path to config".into()))?
        .to_owned()
        + "/makemake/templates/"
        + name)
}

/// Prints colorful help to the stdout.
fn help() {
    let v: Option<&str> = option_env!("CARGO_PKG_VERSION");
    printcln!(
        "Welcome in {'g i}makemake{'_} by {}{'_}
Version {}

{'g}Usage:
  {'w}makemake <template name> {'gr}[options]{'_}
    loads template

  {'w}makemake {'gr}[options]{'_}
    bahaves according to the options, with no options shows this help

{'g}Options:
  {'y}-h  --help{'_}
    shows this help

  {'y}-c  --create {'w}<template name>{'_}
    creates new template with the name

  {'y}-cf  --create-from {'w}<template name> <template surce directory>{'_}
    creates new template from the directory with the name

  {'y}--load {'w}<template name>{'_}
    loads the given template

  {'y}-lt  --load-to {'w}<template name> <destination directory>{'_}
    loads the given template into the destination directory (will be created
    if it doesn't exist)

  {'y}-r  --remove {'w}<template name>{'_}
    removes the given template

  {'y}-l  --list{'_}
    lists all the template names

  {'y}-D{'w}<variable name>{'gr}[=value]{'_}
    defines/redefines a variable

  {'y}-e  --edit {'w}<template name>{'_}
    loads template source to this directory

  {'y}-ei --edit-in {'w}<template name> <directory>{'_}
    loads template source to the given direcotry

  {'y}-p  --prompt {'w}<yes | no | ask>{'_}
    sets the default answer when prompting. 'yes' will always answer with 'y',
    'no' will always answer with 'n', 'ask' is default - always ask

  {'y}-py{'_}, {'y}-pn{'_}, {'y}-pa{'_}
    same as '-p yes', '-p no' and '-p ask' respectively

In case that multiple options that specify the same setting are used, only
the last is taken into account. e.g. 'makemake vscm -e vscm' is same as
'makemake -e vscm', 'makemake vscm -py -pa' is same as 'makemake vscm' and
'makemake vscm -c vscm' is same as 'makemake -c vscm'
",
        // BonnyAD9 gradient in 3 strings
        termal::gradient("BonnyAD9", (250, 50, 170), (180, 50, 240)),
        v.unwrap_or("unknown")
    );
}
