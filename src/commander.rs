use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path, process::Command};

use crate::err::{Error, Result};

pub fn run_command<P1, P2>(cmd: &str, cwd: P1, pwd: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let com = Command::new(cwd.as_ref().join(cmd))
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
