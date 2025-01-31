use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
    fs::{self, create_dir_all, read_dir, read_link, File},
    io::{BufReader, BufWriter},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use utf8_chars::BufReadCharsExt;

use crate::{
    commander::run_command, err::Result, parser::parse, writer::ToFmtWrite,
};

#[derive(Serialize, Deserialize)]
struct MakeConfig<'a> {
    #[serde(default, rename = "preCommand")]
    pre_command: Option<String>,
    #[serde(default, rename = "postCommand")]
    post_command: Option<String>,
    #[serde(default, rename = "expandVariables")]
    expand_variables: bool,
    #[serde(default)]
    files: HashMap<PathBuf, MakeInfo>,
    #[serde(default)]
    vars: HashMap<Cow<'a, str>, Cow<'a, str>>,
    #[serde(skip)]
    template_path: PathBuf,
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

#[derive(Copy, Clone)]
pub struct ExpandContext<'a> {
    pub vars: &'a HashMap<Cow<'a, str>, Cow<'a, str>>,
    pub template_dir: &'a Path,
}

pub fn create_template<P1, P2>(src: P1, out: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    copy_dir(src, out)
}

pub fn load_template<'a, P1, P2>(
    src: P1,
    dst: P2,
    vars: HashMap<Cow<'a, str>, Cow<'a, str>>,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let src = src.as_ref();
    let dst: Cow<Path> = dst
        .as_ref()
        .canonicalize()
        .map(|p| p.into())
        .unwrap_or(dst.as_ref().into());
    let dst = dst.as_ref();
    let conf = src.join("makemake.json");

    if conf.try_exists()? {
        let conf = File::open(conf)?;
        let mut conf: MakeConfig = serde_json::from_reader(conf)?;
        conf.template_path = src.to_owned();

        create_dir_all(dst)?;
        conf.init(vars, dst)?;

        let run_cmd = |c: &String| {
            let mut cmd = String::new();
            expand(conf.context(), &mut c.chars().map(Ok), &mut cmd)?;
            run_command(&cmd, src, dst, &conf.vars)
        };

        conf.pre_command.as_ref().map(run_cmd).unwrap_or(Ok(()))?;

        conf.make_dir(src, dst)?;

        conf.post_command.as_ref().map(run_cmd).unwrap_or(Ok(()))?;
        Ok(())
    } else {
        copy_dir(src, dst)
    }
}

impl<'a> MakeConfig<'a> {
    fn load_internal_variables<P>(
        vars: &mut HashMap<Cow<'a, str>, Cow<'a, str>>,
        dst: P,
    ) where
        P: AsRef<Path>,
    {
        if let Some(dname) = dst.as_ref().file_name() {
            vars.entry("_PDIR".into())
                .or_insert(dname.to_string_lossy().into_owned().into());
        }

        #[cfg(target_os = "linux")]
        {
            vars.entry("_LINUX".into()).or_insert("linux".into());
            vars.entry("_OS".into()).or_insert("linux".into());
        }
        #[cfg(target_os = "windows")]
        {
            vars.entry("_WINDOWS".into()).or_insert("windows".into());
            vars.entry("_OS".into()).or_insert("windows".into());
        }
        #[cfg(target_os = "macos")]
        {
            vars.entry("_MACOS".into()).or_insert("macos".into());
            vars.entry("_OS".into()).or_insert("macos".into());
        }
        #[cfg(target_os = "ios")]
        {
            vars.entry("_IOS".into()).or_insert("ios".into());
            vars.entry("_OS".into()).or_insert("ios".into());
        }
        #[cfg(target_os = "freebsd")]
        {
            vars.entry("_FREEBSD".into()).or_insert("freebsd".into());
            vars.entry("_OS".into()).or_insert("freebsd".into());
        }
    }

    fn init<P>(
        &mut self,
        mut vars: HashMap<Cow<'a, str>, Cow<'a, str>>,
        dst: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        Self::load_internal_variables(&mut vars, dst);

        if self.expand_variables {
            self.expand_variables(&vars)?;
        }
        self.vars.extend(vars);

        Ok(())
    }

    fn context(&self) -> ExpandContext<'_> {
        ExpandContext {
            vars: &self.vars,
            template_dir: &self.template_path,
        }
    }

    fn expand_variables(
        &mut self,
        vars: &HashMap<Cow<'a, str>, Cow<'a, str>>,
    ) -> Result<()> {
        for v in self.vars.values_mut() {
            let mut res = String::new();
            expand(
                ExpandContext {
                    vars,
                    template_dir: &self.template_path,
                },
                &mut v.chars().map(Ok),
                &mut res,
            )?;
            *v = res.into();
        }

        Ok(())
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
                    let adr = read_link(src)?;
                    symlink(adr, dst)?;
                }
            }
        } else {
            let adr = read_link(src)?;
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
        let srel = src.strip_prefix(rsrc)?;

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
        let srel = src.strip_prefix(rsrc)?;

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
        expand(self.context(), src, dst)
    }

    fn make_name(
        &self,
        info: &FileInfo,
        path: &mut PathBuf,
    ) -> Result<MakeType> {
        if !info.name.is_empty() {
            let mut name = String::new();
            self.expand(&mut info.name.chars().map(Ok), &mut name)?;
            if name.is_empty() {
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

pub fn expand<I, W>(ctx: ExpandContext, src: &mut I, dst: &mut W) -> Result<()>
where
    I: Iterator<Item = Result<char>>,
    W: Write,
{
    while let Some(c) = src.next().transpose()? {
        if c != '$' {
            dst.write_char(c)?;
            continue;
        }

        if let Some(c) = src.next().transpose()? {
            if c != '{' {
                dst.write_char('$')?;
                dst.write_char(c)?;
                continue;
            }

            parse(src)?.eval(dst, ctx)?;
        }
    }

    Ok(())
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
