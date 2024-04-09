use std::{borrow::Cow, ffi::OsStr, os::unix::ffi::OsStrExt, path::Path, process::Command};

use crate::err::{Error, Result};

pub fn run_command<P1, P2>(cmd: &str, cwd: P1, pwd: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let (program, args) = parse_command(cmd);
    let mut program: Cow<Path> = Path::new(program).into();
    if program.starts_with(".") || program.components().count() > 1 {
        program = cwd.as_ref().join(program).into();
    }

    let com = Command::new(program.as_ref())
        .args(args)
        .current_dir(pwd)
        .output()?;
    if !com.status.success() {
        let s = OsStr::from_bytes(&com.stdout)
            .to_string_lossy()
            .into_owned();
        return Err(Error::CommandUnsuccessful {
            cmd: cmd.to_owned(),
            stderr: s,
        });
    }
    Ok(())
}

fn parse_command(cmd: &str) -> (&str, Vec<&str>) {
    if let Some((cmd, args)) = cmd.split_once(|c: char| c.is_ascii_whitespace()) {
        (cmd, args.split_ascii_whitespace().collect())
    } else {
        (cmd, vec![])
    }
}
