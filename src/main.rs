use args::{Action, Args, Yna};
use config::Config;
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

use crate::{config::Alias, err::Error};

mod args;
mod ast;
mod commander;
mod config;
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
    let args = Args::parse(args.iter().skip(1).into())?;

    args.check_unused();

    // Do what the arguments specify
    match args.get_action() {
        Action::Create => create(args)?,
        Action::Alias => alias(args)?,
        Action::Config => configure(args)?,
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

/// Creates new template with the name `name` from the directory `src` in the
/// default template folder.
fn alias(args: Args) -> Result<()> {
    let template = args.get_template()?;
    let alias_name = args.get_alias()?;
    let mut conf = load_config()?;

    if conf.aliases.contains_key(alias_name)
        && !prompt_yn(
            &format!(
                "Alias with the name '{alias_name}' already exists.\n\
                Do you want to overwire it? [y/N]: "
            ),
            args.prompt_answer,
        )?
    {
        return Ok(());
    }

    let alias = Alias {
        template: template.to_owned(),
        vars: args
            .vars
            .into_iter()
            .map(|(k, v)| (k.into_owned().into(), v.into_owned().into()))
            .collect(),
    };

    conf.aliases.insert(alias_name.to_owned(), alias);

    save_config(&conf)
}

fn configure(args: Args) -> Result<()> {
    let mut conf = load_config()?;

    for (k, v) in args.vars {
        if let Some(k) = k.strip_prefix('-') {
            conf.vars.remove(k);
        } else if let Some(k) = k.strip_prefix('+') {
            conf.vars.insert(k.to_owned().into(), v.into_owned().into());
        } else {
            conf.vars
                .insert(k.into_owned().into(), v.into_owned().into());
        }
    }

    save_config(&conf)
}

/// Loads template with the name `src` to the directory `dest`. `vars` can
/// add/override variables in the template config file.
fn load(mut args: Args) -> Result<()> {
    let mut name = args.get_template()?;
    let conf = load_config()?;

    if let Some(a) = conf.aliases.get(name) {
        name = &a.template;
        for (k, v) in &a.vars {
            args.vars.entry(k.clone()).or_insert(v.clone());
        }
    }

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

    let mut vars = args.vars;
    for (k, v) in conf.vars {
        vars.entry(k).or_insert(v);
    }

    load_template(template, dest, vars)
}

/// Deletes template with the name `name`
fn remove(args: Args) -> Result<()> {
    let mut conf = load_config()?;
    if conf.aliases.remove(args.get_template()?).is_some() {
        save_config(&conf)
    } else {
        remove_dir_all(get_template_dir(args.get_template()?)?)?;
        Ok(())
    }
}

/// Copies the template with the name `name` to the directory `dest`.
fn edit(args: Args) -> Result<()> {
    let name = args.get_template()?;
    let dest = args.get_directory();

    let template = get_template_dir(name)?;
    if !template.exists() {
        let conf = load_config()?;
        if let Some(n) = conf.aliases.get(name) {
            return Err(Error::Msg(
                format!(
                    "{name} is not a template, it is alias for the template \
                    {}",
                    n.template
                )
                .into(),
            ));
        }
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

    let conf = load_config()?;
    for (n, a) in conf.aliases {
        print!("{n} : {}", a.template);
        for (n, v) in a.vars {
            if v.is_empty() {
                print!(" -D{n}",);
            } else {
                print!(" -D{n}={v}");
            }
        }
        println!();
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

fn load_config() -> Result<Config> {
    let mut config =
        config_dir().ok_or(Error::Msg("Can't get config directory".into()))?;
    config.push("config.json");
    if config.try_exists()? {
        Config::from_file(config)
    } else {
        Ok(Config::default())
    }
}

fn save_config(conf: &Config) -> Result<()> {
    let mut config =
        config_dir().ok_or(Error::Msg("Can't get config directory".into()))?;
    config.push("config.json");
    conf.to_file(config)
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
  {'c}makemake {'w}<template name> {'gr}[options]{'_}
    Behaves according to the options, by default loads template. Aliases
    have priority over templates.

  {'c}makemake {'gr}[options]{'_}
    Bahaves according to the options, with no options shows this help.

{'g}Options:
  {'y}-h  -?  --help{'_}
    Shows this help.

  {'y}-c  --create{'_}
    Creates new template.

  {'y}-r  --remove{'_}
    Removes the template or alias. Aliases have priority over templates.

  {'y}-l  --list{'_}
    Lists all the template names and aliases.

  {'y}-D{'w}<variable name>{'gr}[=value]{'_}
    Defines/redefines a variable.

  {'y}-e  --edit {'w}<template name>{'_}
    Loads template source to this directory. If the directory is destination
    directory and it doesn't exist, it will be created.

  {'y}-a  --alias {'w}<alias name>{'_}
    Creates alias for the given template. All wariables set with {'y}-D{'_}
    will be automatically set when using the alias. You cannot create alias
    for alias.

  {'y}-C  --config  --configure{'_}
    Adds all the variables to the global configuration. If the variable name
    is preceded with '-', it is removed from the global configuration. If the
    variable name starts with '+', it will be added to the global
    configuration.

  {'y}-p  --prompt {'w}<yes | no | ask>{'_}
    Sets the default answer when prompting. 'yes' will always answer with 'y',
    'no' will always answer with 'n', 'ask' is default - always ask. Can be
    overriden.

  {'y}-py{'_}, {'y}-pn{'_}, {'y}-pa{'_}
    Same as '{'y}-p {'w}yes{'_}', '{'y}-p {'w}no{'_}' and '{'y}-p {'w}ask{'_}'\
    respectively.

  {'y}-d  --directory {'w}<path to directory>{'_}
    Sets the relevant directory path. This is cwd by default.

  {'y}-t  --template {'w}<template name>{'_}
    Sets the template, this can be used for templates which name starts with
    '-'.

  {'y}--load {'w}<template name>{'_}
    Loads the given template.

  {'y}-cf  --create-from {'w}<template name> <template surce directory>{'_}
    Creates new template from the directory with the name. (Equivalent to
    '{'y}-c -t {'w}<template name> {'y}-d {'w}<template source directory>{'_}'\
    .)

  {'y}-lt  --load-to {'w}<template name> <destination directory>{'_}
    Loads the given template into the destination directory (will be created
    if it doesn't exist). (Equivalent to
    '{'y}--load {'w}<temlate name> {'y}-d {'w}<destination directory>{'_}'.)

  {'y}-ei --edit-in {'w}<template name> <directory>{'_}
    Loads template source to the given direcotry. (Equivalent to
    '{'y}-e -t {'w}<template name> {'y}-d {'w}<template source directory>{'_}'\
    .)

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
