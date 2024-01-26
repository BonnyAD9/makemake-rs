use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
    fs::{self, create_dir_all, read_dir, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use result::OptionResultExt;
use serde::{Deserialize, Serialize};
use utf8_chars::BufReadCharsExt;

use crate::{
    err::{Error, Result},
    parser::parse,
    writer::ToFmtWrite,
};

#[derive(Serialize, Deserialize)]
struct MakeConfig {
    #[serde(default)]
    files: HashMap<PathBuf, MakeInfo>,
    #[serde(default)]
    vars: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MakeInfo {
    TypeOnly(MakeType),
    Info(FileInfo),
}

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
enum MakeType {
    #[default]
    Copy,
    Make,
    Ignore,
}

#[derive(Clone, Serialize, Deserialize)]
struct FileInfo {
    #[serde(default)]
    action: MakeType,
    #[serde(default)]
    name: String,
}

pub fn create_template<P>(src: P, out: P) -> Result<()>
where
    P: AsRef<Path>,
{
    copy_dir(src, out)
}

pub fn load_template<P1, P2>(
    src: P1,
    dst: P2,
    vars: HashMap<String, String>,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let src = src.as_ref();
    let dst = dst.as_ref();
    let conf = src.join("makemake.json");

    if conf.try_exists()? {
        let conf = File::open(conf)?;
        let mut conf: MakeConfig = serde_json::from_reader(conf)?;
        conf.load_internal_variables();
        conf.vars.extend(vars);
        conf.make_dir(src, dst)?;
        Ok(())
    } else {
        copy_dir(src, dst)
    }
}

impl MakeConfig {
    fn load_internal_variables(&mut self) {
        #[cfg(target_os = "linux")]
        self.vars.insert("_LINUX".to_owned(), "true".to_owned());
        #[cfg(target_os = "windows")]
        self.vars.insert("_WINDOWS".to_owned(), "true".to_owned());
        #[cfg(target_os = "macos")]
        self.vars.insert("_MACOS".to_owned(), "true".to_owned());
        #[cfg(target_os = "ios")]
        self.vars.insert("_IOS".to_owned(), "true".to_owned());
        #[cfg(target_os = "freebsd")]
        self.vars.insert("_FREEBSD".to_owned(), "true".to_owned());
        self.vars.insert("_".to_owned(), "".to_owned());
    }

    fn make_dir<P>(&self, rsrc: P, rdst: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut dirs: Vec<(Cow<_>, Cow<_>)> =
            vec![(rsrc.as_ref().into(), rdst.as_ref().into())];

        while let Some((src, dst)) = dirs.pop() {
            let meta = src.metadata()?;
            if meta.is_file() {
                // src is always subpath of rsrc
                // let srel = diff_paths(&src, &rsrc).unwrap();
                let srel = src.strip_prefix(&rsrc)?;

                if let Some(info) = self.files.get(srel) {
                    self.make_file_name(
                        info,
                        src.into_owned(),
                        dst.into_owned(),
                    )?;
                } else {
                    fs::copy(src, dst)?;
                }
            } else if meta.is_dir() {
                create_dir_all(&dst)?;

                for f in read_dir(src)? {
                    let f = f?;
                    let path = f.path();
                    let name = f.file_name();
                    dirs.push((path.into(), dst.join(name).into()))
                }
            } else if meta.is_symlink() {
                return Err(Error::Unsupported(
                    "symlinks in templates are not supported",
                ));
            }
        }

        Ok(())
    }

    fn make_file_name(
        &self,
        info: &MakeInfo,
        src: PathBuf,
        mut dst: PathBuf,
    ) -> Result<()> {
        let action = match info {
            MakeInfo::TypeOnly(a) => *a,
            MakeInfo::Info(i) => {
                if i.name.len() != 0 {
                    let mut name = String::new();
                    self.expand(
                        &mut i.name.chars().map(|a| Ok(a)),
                        &mut name,
                    )?;
                    if name.len() == 0 {
                        MakeType::Ignore
                    } else {
                        dst.set_file_name(name);
                        i.action
                    }
                } else {
                    i.action
                }
            }
        };

        match action {
            MakeType::Copy => _ = fs::copy(src, dst)?,
            MakeType::Make => {
                let mut src = BufReader::new(File::open(src)?);
                self.expand(
                    &mut src.chars().map(|c| Ok(c?)),
                    &mut ToFmtWrite(BufWriter::new(File::create(dst)?)),
                )?;
            }
            MakeType::Ignore => {}
        }

        Ok(())
    }

    fn expand<I, W>(&self, src: &mut I, dst: &mut W) -> Result<()>
    where
        I: Iterator<Item = Result<char>>,
        W: Write,
    {
        while let Some(c) = src.next().invert()? {
            if c != '$' {
                dst.write_char(c)?;
                continue;
            }

            if let Some(c) = src.next().invert()? {
                if c != '{' {
                    dst.write_char('$')?;
                    dst.write_char(c)?;
                    continue;
                }

                parse(src)?.eval(dst, &self.vars)?;
            }
        }

        Ok(())
    }
}

pub fn copy_dir<P>(src: P, dst: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut dirs: Vec<(Cow<_>, Cow<_>)> =
        vec![(src.as_ref().into(), dst.as_ref().into())];

    while let Some((src, dst)) = dirs.pop() {
        let meta = src.metadata()?;
        if meta.is_file() {
            fs::copy(src, dst)?;
        } else if meta.is_dir() {
            create_dir_all(&dst)?;

            for f in read_dir(src)? {
                let f = f?;
                let path = f.path();
                let name = f.file_name();
                dirs.push((path.into(), dst.join(name).into()));
            }
        } else if meta.is_symlink() {
            return Err(Error::Unsupported(
                "symlinks in templates are not supported",
            ));
        }
    }

    Ok(())
}
