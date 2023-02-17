use dirs::config_dir;
use eyre::{Report, Result};
use maker::{create_template, load_tempalte};
use std::{
    env,
    fs::{read_dir, remove_dir_all},
    io::{stdin, stdout, Write},
    path::Path,
};
use Action::*;

mod maker;

enum Action<'a> {
    Help,
    Create((&'a str, &'a str)),
    Load((&'a str, &'a str)),
    List,
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();

    let mut args = args[1..].iter();
    let mut action = Help;

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
            _ => action = Load((&arg, "./")),
        }
    }

    match action {
        Create(n) => create(n.0, n.1)?,
        Load(n) => load(n.0, n.1)?,
        Help => help(),
        List => list()?,
    }

    Ok(())
}

fn create(name: &str, src: &str) -> Result<()> {
    let tdir = get_template_dir(name)?;

    if Path::new(&tdir).exists() {
        if prompt_yn(&format!(
            "Template with the name '{name}' already \
            exists.\nDo you want to overwrite it? [y/N]: "
        ))?
        .is_none()
        {
            return Ok(());
        }
        remove_dir_all(&tdir)?;
    }

    create_template(src, &tdir)
}

fn load(name: &str, dest: &str) -> Result<()> {
    // true if the directory exists and isn't empty
    if read_dir(dest).ok().and_then(|mut d| d.next()).is_some() {
        if prompt_yn(&format!(
            "the directory {dest} is not empty.\n\
            Do you want to load the template anyway? [y/N]: "
        ))?
        .is_none()
        {
            return Ok(());
        }
    }

    load_tempalte(&get_template_dir(name)?, dest)
}

fn list() -> Result<()> {
    for f in read_dir(get_template_dir("")?)? {
        println!(
            "{}",
            f?.file_name().to_str().ok_or(Report::msg("invalid name"))?
        );
    }
    Ok(())
}

fn prompt_yn(prompt: &str) -> Result<Option<()>> {
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

fn help() {
    println!(
        "Welcome in {g}{i}makemake{r} by {}{}{}

{g}Usage:{r}
  {w}makemake{r} {w}[template name]{r}
    loads template
  {w}makemake{r} {d}[options]{r}

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

  {y}-l  --list{r}
    lists all the template names
",
        // BonnyAD9 gradient in 3 strings
        "\x1b[38;2;250;50;170mB\x1b[38;2;240;50;180mo\x1b[38;2;230;50;190mn",
        "\x1b[38;2;220;50;200mn\x1b[38;2;210;50;210my\x1b[38;2;200;50;220mA",
        "\x1b[38;2;190;50;230mD\x1b[38;2;180;50;240m9\x1b[0m",
        g = "\x1b[92m", // green
        i = "\x1b[23m", // italic
        r = "\x1b[0m",  // reset
        w = "\x1b[97m", // white
        d = "\x1b[90m", // dark gray
        y = "\x1b[93m"  // yellow
    );
}
