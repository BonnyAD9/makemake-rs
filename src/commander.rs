use std::{borrow::Cow, collections::HashMap, path::Path, process::Command};

use crate::err::{Error, Result};

pub fn run_command<P1, P2>(
    cmd: &str,
    cwd: P1,
    pwd: P2,
    vars: &HashMap<Cow<str>, Cow<str>>,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let args = parse_command(cmd)?;
    let program = args.first().unwrap();
    let args = &args[1..];

    let mut program: Cow<Path> = Path::new(program).into();
    if program.starts_with(".") || program.components().count() > 1 {
        program = cwd.as_ref().join(program).into();
    }

    let com = Command::new(program.as_ref())
        .args(args)
        .current_dir(pwd)
        .envs(vars.iter().map(|(k, v)| (k.as_ref(), v.as_ref())))
        .output()?;
    if !com.status.success() {
        let s = String::from_utf8_lossy(&com.stderr).into_owned();
        return Err(Error::CommandUnsuccessful {
            cmd: cmd.to_owned(),
            stderr: s,
        });
    }
    Ok(())
}

fn parse_command(cmd: &str) -> Result<Vec<String>> {
    let cmd = shell_words::split(cmd)?;
    if cmd.is_empty() {
        Err(Error::Msg("Invalid command, missing program.".into()))
    } else {
        Ok(cmd)
    }
}
