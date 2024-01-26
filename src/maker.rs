use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
    fs::{self, create_dir_all, read_dir, read_link, File},
    io::{BufReader, BufWriter},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use result::OptionResultExt;
use serde::{Deserialize, Serialize};
use utf8_chars::BufReadCharsExt;

use crate::{err::Result, parser::parse, writer::ToFmtWrite};

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
    Auto,
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

pub fn create_template<P1, P2>(src: P1, out: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
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
        self.vars.insert("_LINUX".to_owned(), "linux".to_owned());
        #[cfg(target_os = "windows")]
        self.vars
            .insert("_WINDOWS".to_owned(), "windows".to_owned());
        #[cfg(target_os = "macos")]
        self.vars.insert("_MACOS".to_owned(), "macos".to_owned());
        #[cfg(target_os = "ios")]
        self.vars.insert("_IOS".to_owned(), "ios".to_owned());
        #[cfg(target_os = "freebsd")]
        self.vars
            .insert("_FREEBSD".to_owned(), "freebsd".to_owned());
    }

    fn make_dir<P>(&self, rsrc: P, rdst: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut dirs: Vec<(Cow<_>, Cow<_>)> =
            vec![(rsrc.as_ref().into(), rdst.as_ref().into())];

        while let Some((src, dst)) = dirs.pop() {
            let meta = src.symlink_metadata()?;
            if meta.is_symlink() {
                self.make_symlink_name(src, &rsrc, dst.into_owned())?;
            } else if meta.is_file() {
                self.make_file_name(src, &rsrc, dst.into_owned())?;
            } else if meta.is_dir() {
                self.make_dir_name(&mut dirs, src, &rsrc, dst.into_owned())?;
            }
        }

        Ok(())
    }

    fn make_symlink_name<P1, P2>(
        &self,
        src: P1,
        rsrc: P2,
        mut dst: PathBuf,
    ) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let src = src.as_ref();
        let rsrc = rsrc.as_ref();

        // src is always subpath of rsrc
        // let srel = diff_paths(&src, &rsrc).unwrap();
        let srel = src.strip_prefix(rsrc)?;

        if let Some(info) = self.files.get(srel) {
            let action = match info {
                MakeInfo::TypeOnly(a) => *a,
                MakeInfo::Info(i) => self.make_name(i, &mut dst)?,
            };

            match action {
                MakeType::Ignore => {}
                _ => {
                    let adr = read_link(&src)?;
                    symlink(adr, dst)?;
                }
            }
        } else {
            let adr = read_link(&src)?;
            symlink(adr, dst)?;
        }

        Ok(())
    }

    fn make_file_name<P1, P2>(
        &self,
        src: P1,
        rsrc: P2,
        mut dst: PathBuf,
    ) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let src = src.as_ref();
        let rsrc = rsrc.as_ref();

        // src is always subpath of rsrc
        // let srel = diff_paths(&src, &rsrc).unwrap();
        let srel = src.strip_prefix(&rsrc)?;

        if let Some(info) = self.files.get(srel) {
            let action = match info {
                MakeInfo::TypeOnly(a) => *a,
                MakeInfo::Info(i) => self.make_name(i, &mut dst)?,
            };

            match action {
                MakeType::Copy | MakeType::Auto => _ = fs::copy(src, dst)?,
                MakeType::Make => {
                    let mut src = BufReader::new(File::open(src)?);
                    self.expand(
                        &mut src.chars().map(|c| Ok(c?)),
                        &mut ToFmtWrite(BufWriter::new(File::create(dst)?)),
                    )?;
                }
                MakeType::Ignore => {}
            }
        } else {
            fs::copy(src, dst)?;
        }

        Ok(())
    }

    fn make_dir_name<P1, P2>(
        &self,
        dirs: &mut Vec<(Cow<Path>, Cow<Path>)>,
        src: P1,
        rsrc: P2,
        mut dst: PathBuf,
    ) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let src = src.as_ref();
        let rsrc = rsrc.as_ref();

        // src is always subpath of rsrc
        // let srel = diff_paths(&src, &rsrc).unwrap();
        let srel = src.strip_prefix(&rsrc)?;

        if let Some(info) = self.files.get(srel) {
            let action = match info {
                MakeInfo::TypeOnly(a) => *a,
                MakeInfo::Info(i) => self.make_name(i, &mut dst)?,
            };
            match action {
                MakeType::Copy => copy_dir(src, dst)?,
                MakeType::Auto | MakeType::Make => {
                    query_dir_copy(src, dst, dirs)?
                }
                MakeType::Ignore => {}
            }
        } else {
            query_dir_copy(src, dst, dirs)?;
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

    fn make_name(
        &self,
        info: &FileInfo,
        path: &mut PathBuf,
    ) -> Result<MakeType> {
        if info.name.len() != 0 {
            let mut name = String::new();
            self.expand(&mut info.name.chars().map(|a| Ok(a)), &mut name)?;
            if name.len() == 0 {
                Ok(MakeType::Ignore)
            } else {
                path.set_file_name(name);
                Ok(info.action)
            }
        } else {
            Ok(info.action)
        }
    }
}

pub fn copy_dir<P1, P2>(rsrc: P1, rdst: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let mut dirs: Vec<(Cow<_>, Cow<_>)> =
        vec![(rsrc.as_ref().into(), rdst.as_ref().into())];

    while let Some((src, dst)) = dirs.pop() {
        let meta = src.symlink_metadata()?;
        if meta.is_symlink() {
            let adr = read_link(&src)?;
            symlink(adr, dst)?;
        } else if meta.is_file() {
            fs::copy(src, dst)?;
        } else if meta.is_dir() {
            query_dir_copy(src, dst, &mut dirs)?;
        }
    }

    Ok(())
}

fn query_dir_copy<P1, P2>(
    src: P1,
    dst: P2,
    queue: &mut Vec<(Cow<Path>, Cow<Path>)>,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let dst = dst.as_ref();

    create_dir_all(dst)?;

    for f in read_dir(src)? {
        let f = f?;
        let path = f.path();
        let name = f.file_name();
        queue.push((path.into(), dst.join(name).into()));
    }

    Ok(())
}
