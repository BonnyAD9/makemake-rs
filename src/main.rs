use dirs::config_dir;
use eyre::{Report, Result};
use maker::{copy_dir, create_template, load_template};
use std::{
    collections::HashMap,
    env,
    fs::{read_dir, remove_dir_all},
    io::{stdin, stdout, Write},
    path::Path
};
use Action::*;

mod maker;

enum Action<'a> {
    Help,
    Create((&'a str, &'a str)),
    Load((&'a str, &'a str)),
    Remove(&'a str),
    List,
    Edit((&'a str, &'a str)),
}

enum PromptAnswer {
    Yes,
    No,
    Ask
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();

    let mut args = args[1..].iter();
    let mut action = Help;

    let mut vars = HashMap::<String, String>::new();

    let mut answer = PromptAnswer::Ask;

    // process the arguments
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => action = Help,
            "-c" | "--create" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected new template name after option '{arg}'"
                )))?;
                action = Create((name, "./"));
            }
            "-cf" | "--create-from" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected new template name and source directory \
                    after option '{arg}'"
                )))?;
                let src = args.next().ok_or(Report::msg(format!(
                    "Expected source directory adter option '{arg}' \
                    and template name"
                )))?;
                action = Create((name, src));
            }
            "--load" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected existing template name after option '{arg}'"
                )))?;
                action = Load((name, "./"))
            }
            "-lt" | "--load-to" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected existing template name and destination \
                    directory after option '{arg}'"
                )))?;
                let dest = args.next().ok_or(Report::msg(format!(
                    "Expected destination directory adter option '{arg}' \
                    and template name"
                )))?;
                action = Load((name, dest));
            }
            "-l" | "--list" => action = List,
            "-r" | "--remove" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected existing template name after option {arg}"
                )))?;
                action = Remove(name);
            }
            "-e" | "--edit" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected existing template name after option {arg}"
                )))?;
                action = Edit((name, "./"));
            }
            "-ei" | "--edit-in" => {
                let name = args.next().ok_or(Report::msg(format!(
                    "Expected existing template name after option {arg}"
                )))?;
                let dir = args.next().ok_or(Report::msg(format!(
                    "Expected directory after option {arg} and template name"
                )))?;
                action = Edit((name, dir));
            },
            "-p" | "--prompt-answer" => {
                let answ = args.next().ok_or(Report::msg(format!(
                    "Expected 'yes', 'no' or 'ask' after option {arg}"
                )))?;
                match answ.to_lowercase().as_str() {
                    "yes" => answer = PromptAnswer::Yes,
                    "no" => answer = PromptAnswer::No,
                    "ask" => answer = PromptAnswer::Ask,
                    _ => return Err(Report::msg(format!(
                        "Expected 'yes', 'no' or 'ask' after option {arg}"
                    ))),
                }
            },
            "-py" => answer = PromptAnswer::Yes,
            "-pn" => answer = PromptAnswer::No,
            "-pa" => answer = PromptAnswer::Ask,
            _ => {
                if arg.starts_with("-D") {
                    let arg = &arg[2..];
                    if let Some(p) =
                        arg.as_bytes().iter().position(|b| (*b as char) == '=')
                    {
                        vars.insert(
                            arg[..p].to_owned(),
                            arg[(p + 1)..].to_owned(),
                        );
                    } else {
                        vars.insert(arg.to_owned(), " ".to_owned());
                    }
                    continue;
                }
                if arg.starts_with("-") {
                    return Err(Report::msg(format!(
                        "Invalid option '{}'. If you ment to load template \
                        use the option '--load' for templates that start with \
                        '-'",
                        arg
                    )))
                }
                action = Load((&arg, "./"))
            }
        }
    }

    // Do what the arguments specify
    match action {
        Create(n) => create(n.0, n.1, answer)?,
        Load(n) => load(n.0, n.1, vars, answer)?,
        Remove(n) => remove(n)?,
        Edit(n) => edit(n.0, n.1, answer)?,
        Help => help(),
        List => list()?,
    }

    Ok(())
}

/// Creates new template with the name `name` from the directory `src` in the
/// default template folder.
fn create(name: &str, src: &str, answ: PromptAnswer) -> Result<()> {
    let tdir = get_template_dir(name)?;

    if Path::new(&tdir).exists() {
        if prompt_yn(&format!(
            "Template with the name '{name}' already \
            exists.\nDo you want to overwrite it? [y/N]: "
        ), answ)?
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
fn load(name: &str, dest: &str, vars: HashMap<String, String>, answ: PromptAnswer) -> Result<()> {
    // true if the directory exists and isn't empty
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some() {
        if prompt_yn(&format!(
            "the directory {dest} is not empty.\n\
            Do you want to load the template anyway? [y/N]: "
        ), answ)?
        .is_none()
        {
            return Ok(());
        }
    }

    load_template(&get_template_dir(name)?, dest, vars)
}

/// Deletes template with the name `name`
fn remove(name: &str) -> Result<()> {
    remove_dir_all(get_template_dir(name)?)?;
    Ok(())
}

/// Copies the template with the name `name` to the directory `dest`.
fn edit(name: &str, dest: &str, answ: PromptAnswer) -> Result<()> {
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some() {
        if prompt_yn(&format!(
            "the directory {dest} is not empty.\n\
            Do you want to load the template source anyway? [y/N]: "
        ), answ)?
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
            f?.file_name().to_str().ok_or(Report::msg("invalid name"))?
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
        PromptAnswer::Ask => {},
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
        _ => Err(Report::msg(format!("Invalid option {conf}"))),
    };
}

/// Gets the directory in which the template with the name `name` is stored.
fn get_template_dir(name: &str) -> Result<String> {
    let config =
        config_dir().ok_or(Report::msg("Can't get config directory"))?;

    Ok(config
        .to_str()
        .ok_or(Report::msg("Invalid path to config"))?
        .to_owned()
        + "/makemake/templates/"
        + name)
}

/// Prints colorful help to the stdout.
fn help() {
    let v: Option<&str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "Welcome in {g}{i}makemake{r} by {}{}{}
Version {}

{g}Usage:{r}
  {w}makemake{r} {w}[template name]{r} {d}[options]{r}
    loads template

  {w}makemake{r} {d}[options]{r}
    bahaves according to the options, with no options shows this help

{g}Options:{r}
  {y}-h  --help{r}
    shows this help

  {y}-c  --create{r} {w}[template name]{r}
    creates new template with the name

  {y}-cf  --create-from {w}[template name] [template surce directory]{r}
    creates new template from the directory with the name

  {y}--load{r} {w}[template name]{r}
    loads the given template

  {y}-lt  --load-to{r} {w}[template name] [destination directory]{r}
    loads the given template into the destination directory (will be created
    if it doesn't exist)

  {y}-r  --remove{r} {w}[template name]{r}
    removes the given template

  {y}-l  --list{r}
    lists all the template names

  {y}-D{w}[variable name]{d}=[value]{r}
    defines/redefines a variable

  {y}-e  --edit{r} {w}[template name]{r}
    loads template source to this directory

  {y}-ei --edit-in{r} {w}[template name] [directory]{r}
    loads template source to the given direcotry

  {y}-p  --prompt{r} {w}yes|no|ask{r}
    sets the default answer when prompting. 'yes' will always answer with 'y',
    'no' will always answer with 'n', 'ask' is default - always ask

  {y}-py{r}, {y}-pn{r}, {y}-pa{r}
    same as '-p yes', '-p no' and '-p ask' respectively

In case that multiple options that specify the same setting are used, only
the last is taken into account. e.g. 'makemake vscm -e vscm' is same as
'makemake -e vscm', 'makemake vscm -py -pa' is same as 'makemake vscm' and
'makemake vscm -c vscm' is same as 'makemake -c vscm'
",
        // BonnyAD9 gradient in 3 strings
        "\x1b[38;2;250;50;170mB\x1b[38;2;240;50;180mo\x1b[38;2;230;50;190mn",
        "\x1b[38;2;220;50;200mn\x1b[38;2;210;50;210my\x1b[38;2;200;50;220mA",
        "\x1b[38;2;190;50;230mD\x1b[38;2;180;50;240m9\x1b[0m",
        v.unwrap_or("unknown"),
        g = "\x1b[92m", // green
        i = "\x1b[23m", // italic
        r = "\x1b[0m",  // reset
        w = "\x1b[97m", // white
        d = "\x1b[90m", // dark gray
        y = "\x1b[93m"  // yellow
    );
}
