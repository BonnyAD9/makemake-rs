use eyre::{Report, Result};
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{copy, create_dir_all, read_dir, File},
    io::{Read, Write},
};
use utf8_read::{
    Char::{Char, NoData},
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

    fn read_while<P: Fn(char) -> bool>(&mut self, p: P) -> Result<String> {
        let mut s = String::new();
        while let Char(c) = self.read()? {
            if !p(c) {
                break;
            }
            s.push(c);
        }
        Ok(s)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum MakeType {
    Copy,
    Make,
}

#[derive(Serialize, Deserialize)]
struct MakeConfig {
    files: HashMap<String, MakeType>,
    vars: HashMap<String, String>,
}

impl MakeConfig {
    fn get_type(&self, file: &str) -> MakeType {
        if let Some(t) = self.files.get(file) {
            *t
        } else {
            MakeType::Copy
        }
    }
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

        let opath = dest.to_owned()
            + "/"
            + f.file_name()
                .to_str()
                .ok_or(Report::msg("invalid file name"))?;

        if f.file_type()?.is_dir() {
            make_dir(fpath, &opath, base, conf)?;
            continue;
        }

        let frel =
            diff_paths(fpath, base).ok_or(Report::msg("Invalid base path"))?;
        let frel = frel.to_str().ok_or(Report::msg("Invalid path"))?;

        match conf.get_type(frel) {
            MakeType::Copy => _ = copy(fpath, opath)?,
            MakeType::Make => {
                let mut rw =
                    CharRW::new(File::open(fpath)?, File::create(opath)?);
                make_file(&mut rw, &conf.vars)?;
            }
        }
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
    let name = rw.read_while(|c| c != '}')?;
    if let Some(val) = vars.get(&name) {
        rw.write_str(&val)?;
    }

    if let Char('}') = rw.cur {
        Ok(())
    } else {
        Err(Report::msg("Expected '}'"))
    }
}

fn copy_dir(from: &str, to: &str) -> Result<()> {
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
