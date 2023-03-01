use eyre::{Report, Result};
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{copy, create_dir_all, read_dir, File},
    io::{Read, Write},
};
use utf8_read::{
    Char::{Char, Eof, NoData},
    Reader,
};

struct CharRW<R: Read, W: Write> {
    reader: Reader<R>,
    writer: W,
    cur: utf8_read::Char,
}

impl<R: Read, W: Write> CharRW<R, W> {
    fn write(&mut self, c: char) -> Result<()> {
        let b = c.to_string();
        self.write_bytes(b.as_bytes())
    }

    fn write_str(&mut self, c: &str) -> Result<()> {
        self.write_bytes(c.as_bytes())
    }

    fn write_bytes(&mut self, b: &[u8]) -> Result<()> {
        if self.writer.write(b)? != b.len() {
            Err(Report::msg("cannot write"))
        } else {
            Ok(())
        }
    }

    fn read(&mut self) -> Result<utf8_read::Char> {
        self.cur = self.reader.next_char()?;
        Ok(self.cur)
    }

    /*
    fn cur(&self) -> Result<char> {
        match self.cur {
            Char(c) => Ok(c),
            _ => Err(Report::msg("End of file reached")),
        }
    }*/

    fn new(r: R, w: W) -> Self {
        CharRW {
            reader: Reader::new(r),
            writer: w,
            cur: NoData,
        }
    }

    fn read_while<P: Fn(char) -> bool>(
        &mut self,
        out: &mut String,
        p: P,
    ) -> Result<()> {
        while let Char(c) = self.cur {
            if !p(c) {
                break;
            }
            out.push(c);
            self.read()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum MakeType {
    Copy,
    Make,
    Ignore,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MakeInfoEnum {
    TypeOnly(MakeType),
    Info(FileInfo),
}

#[derive(Clone, Serialize, Deserialize)]
struct FileInfo {
    #[serde(default = "default_file_info_action")]
    action: MakeType,
    #[serde(default)]
    name: String,
}

fn default_file_info_action() -> MakeType {
    MakeType::Copy
}

#[derive(Serialize, Deserialize)]
struct MakeConfig {
    files: HashMap<String, MakeInfoEnum>,
    vars: HashMap<String, String>,
}

pub fn create_template(src: &str, out: &str) -> Result<()> {
    copy_dir(src, out)
}

pub fn load_template(
    src: &str,
    dest: &str,
    vars: HashMap<String, String>,
) -> Result<()> {
    if let Ok(f) = File::open(src.to_owned() + "/makemake.json") {
        let mut conf: MakeConfig = serde_json::from_reader(f)?;
        conf.vars.extend(vars);
        make_dir(src, dest, src, &conf)
    } else {
        copy_dir(src, dest)
    }
}

fn make_dir(
    src: &str,
    dest: &str,
    base: &str,
    conf: &MakeConfig,
) -> Result<()> {
    create_dir_all(dest)?;

    for f in read_dir(src)? {
        let f = f?;

        let fpath = f.path();
        let fpath = fpath.to_str().ok_or(Report::msg("invalid path"))?;

        let dname = f.file_name();
        let dname = dname.to_str().ok_or(Report::msg("invalid file name"))?;
        let opath = dest.to_owned() + "/" + dname;

        if f.file_type()?.is_dir() {
            make_dir(fpath, &opath, base, conf)?;
            continue;
        }

        let frel =
            diff_paths(fpath, base).ok_or(Report::msg("Invalid base path"))?;
        let frel = frel.to_str().ok_or(Report::msg("Invalid path"))?;

        if let Some(c) = conf.files.get(frel) {
            make_file_name(c, &conf.vars, fpath, dest, dname)?;
        } else {
            copy(fpath, opath)?;
        }
    }

    Ok(())
}

fn make_file_name(
    info: &MakeInfoEnum,
    vars: &HashMap<String, String>,
    src: &str,
    dpath: &str,
    dname: &str,
) -> Result<()> {
    let mut buf = String::new();
    let (action, name) = match info {
        MakeInfoEnum::TypeOnly(a) => (*a, dname),
        MakeInfoEnum::Info(i) => {
            if i.name.len() == 0 {
                (i.action, dname)
            } else {
                make_name(
                    &mut CharRW::new(i.name.as_bytes(), [].as_mut()),
                    vars,
                    &mut buf,
                )?;
                (i.action, buf.as_str())
            }
        }
    };

    let dest = dpath.to_owned() + "/" + name;

    match action {
        MakeType::Copy => _ = copy(src, dest)?,
        MakeType::Make => {
            let mut rw = CharRW::new(File::open(src)?, File::create(dest)?);
            make_file(&mut rw, vars)?;
        }
        MakeType::Ignore => {}
    }
    Ok(())
}

fn make_name<R: Read, W: Write>(rw: &mut CharRW<R, W>, vars: &HashMap<String, String>, out: &mut String) -> Result<()> {
    while let Char(c) = rw.read()? {
        if c == '$' {
            if let Char(c) = rw.read()? {
                if c != '{' {
                    out.push('$');
                    out.push(c);
                    continue;
                }
                rw.read()?;
                make_expr_buf(rw, vars, out)?;
                continue;
            }
        }
        out.push(c);
    }
    Ok(())
}

fn make_file<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
) -> Result<()> {
    while let Char(c) = rw.read()? {
        if c == '$' {
            if let Char(c) = rw.read()? {
                if c != '{' {
                    rw.write('$')?;
                    rw.write(c)?;
                    continue;
                }
                rw.read()?;
                make_expr(rw, &vars)?;
                continue;
            }
        }
        rw.write(c)?;
    }

    Ok(())
}

fn make_expr<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
) -> Result<()> {
    let mut buf = String::new();
    make_expr_buf(rw, vars, &mut buf)?;
    rw.write_str(&buf)
}

fn make_expr_buf<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
    while !matches!(rw.cur, Char('}') | Eof | NoData) {
        read_expr(rw, vars, out)?;
    }
    Ok(())
}

fn read_expr<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
    match rw.cur {
        Char('\'') => read_str_literal(rw, out)?,
        Eof | NoData => return Err(Report::msg("Expected expression")),
        Char(c) => {
            if c.is_whitespace() {
                skip_whitespace(rw)?;
            } else {
                read_variable(rw, vars, out)?;
            }
        }
    };
    Ok(())
}

fn skip_whitespace<R: Read, W: Write>(rw: &mut CharRW<R, W>) -> Result<()> {
    rw.read()?;
    while let Char(c) = rw.cur {
        if !c.is_whitespace() {
            break;
        }
        rw.read()?;
    }
    Ok(())
}

fn read_str_literal<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    out: &mut String,
) -> Result<()> {
    rw.read()?;
    loop {
        match rw.cur {
            Char('\\') => read_escape(rw, out)?,
            Char('\'') => {
                rw.read()?;
                return Ok(());
            }
            Char(c) => out.push(c),
            _ => return Err(Report::msg("literal not ended")),
        }
        rw.read()?;
    }
}

fn read_escape<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    out: &mut String,
) -> Result<()> {
    rw.read()?;
    match rw.cur {
        Char('n') => out.push('\n'),
        Char('r') => out.push('\r'),
        Char('t') => out.push('\t'),
        Char('x') => {
            return Err(Report::msg("the '\\x' escape sequence is reserved"))
        }
        Char('u') => {
            return Err(Report::msg("the '\\u' escape sequence is reserved"))
        }
        Char(c) => {
            if c.is_digit(10) {
                return Err(Report::msg(
                    "the '\\<num>' escape sequence is reserved",
                ));
            }
            out.push(c);
        }
        _ => return Err(Report::msg("Expected escape sequence")),
    };
    rw.read()?;
    Ok(())
}

fn read_variable<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
    let mut name = String::new();
    rw.read_while(&mut name, |c| {
        !c.is_whitespace() && !matches!(c, '?' | ':' | '\'' | '{' | '}' | '$')
    })?;
    if let Some(val) = vars.get(&name) {
        out.push_str(val);
    }
    Ok(())
}

pub fn copy_dir(from: &str, to: &str) -> Result<()> {
    create_dir_all(to)?;

    for f in read_dir(from)? {
        let f = f?;

        let fpath = f.path();
        let fpath = fpath.to_str().ok_or(Report::msg("invalid path"))?;

        let opath = to.to_owned()
            + "/"
            + f.file_name()
                .to_str()
                .ok_or(Report::msg("invalid file name"))?;

        if f.file_type()?.is_dir() {
            copy_dir(fpath, &opath)?;
            continue;
        }

        copy(fpath, opath)?;
    }

    Ok(())
}
