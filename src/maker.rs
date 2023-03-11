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

/// Struct for reading and writing chars from Read and Write traits
/// Remembers the last readed char
struct CharRW<R: Read, W: Write> {
    reader: Reader<R>,
    writer: W,
    /// the last char readed by any of the read methods
    cur: utf8_read::Char,
}

impl<R: Read, W: Write> CharRW<R, W> {
    /// Writes single char `c` to the writer
    fn write(&mut self, c: char) -> Result<()> {
        let b = c.to_string();
        self.write_bytes(b.as_bytes())
    }

    /// Writes string `c` to the writer
    fn write_str(&mut self, c: &str) -> Result<()> {
        self.write_bytes(c.as_bytes())
    }

    /// Writes byte array `b` to the writer
    fn write_bytes(&mut self, b: &[u8]) -> Result<()> {
        if self.writer.write(b)? != b.len() {
            Err(Report::msg("cannot write"))
        } else {
            Ok(())
        }
    }

    /// Reads single char from the reader and stores it in the .cur field.
    /// The readed char is also returned.
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

    /// Creates new `CharRW` from Read `r` and Write `w`.
    fn new(r: R, w: W) -> Self {
        CharRW {
            reader: Reader::new(r),
            writer: w,
            cur: NoData,
        }
    }

    /// Appends chars to `out` while `p` returns `true`.
    /// The first unreaded char is stored in the .cur field.
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

/// Type of action on file
#[derive(Clone, Copy, Serialize, Deserialize)]
enum MakeType {
    Copy,
    Make,
    Ignore,
}

/// Type of data stored about a file in a config file
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MakeInfoEnum {
    TypeOnly(MakeType),
    Info(FileInfo),
}

/// Info about a file in a config file.
#[derive(Clone, Serialize, Deserialize)]
struct FileInfo {
    #[serde(default = "default_file_info_action")]
    action: MakeType,
    #[serde(default)]
    name: String,
}

/// Gets the default value for `FileInfo.action`. Used by serde.
fn default_file_info_action() -> MakeType {
    MakeType::Copy
}

/// Configuration from config file
#[derive(Serialize, Deserialize)]
struct MakeConfig {
    files: HashMap<String, MakeInfoEnum>,
    vars: HashMap<String, String>,
}

/// Copies the template source from `src` to `out`
pub fn create_template(src: &str, out: &str) -> Result<()> {
    copy_dir(src, out)
}

/// Loads the template from its source at `src` into `dest` and expands all
/// expressions using the template configuration and `vars`.
/// `vars` have priority over the variables in template config file
pub fn load_template(
    src: &str,
    dest: &str,
    vars: HashMap<String, String>,
) -> Result<()> {
    if let Ok(f) = File::open(src.to_owned() + "/makemake.json") {
        // Load the template according to its config file
        let mut conf: MakeConfig = serde_json::from_reader(f)?;
        load_internal_variables(&mut conf.vars);
        conf.vars.extend(vars);
        make_dir(src, dest, src, &conf)
    } else {
        // if there is no config file, just copy the template
        copy_dir(src, dest)
    }
}

/// Adds internal variables to `vars`
///
/// #### Used by:
/// `load_template`
fn load_internal_variables(vars: &mut HashMap<String, String>) {
    #[cfg(target_os = "linux")]
    vars.insert("_LINUX".to_owned(), "true".to_owned());
    #[cfg(target_os = "windows")]
    vars.insert("_WINDOWS".to_owned(), "true".to_owned());
    #[cfg(target_os = "macos")]
    vars.insert("_MACOS".to_owned(), "true".to_owned());
    #[cfg(target_os = "ios")]
    vars.insert("_IOS".to_owned(), "true".to_owned());
    #[cfg(target_os = "freebsd")]
    vars.insert("_FREEBSD".to_owned(), "true".to_owned());
    vars.insert("_".to_owned(), "".to_owned());
}

/// Loads template from `src` into `dest` with the configuration in `config`.
/// `base` is used for recursive calling. When calling normally it should be
/// the same as `src`.
///
/// #### Used by:
/// `load_template`, `make_dir`
fn make_dir(
    src: &str,
    dest: &str,
    base: &str,
    conf: &MakeConfig,
) -> Result<()> {
    create_dir_all(dest)?;

    for f in read_dir(src)? {
        let f = f?;

        // Path to the source file/directory
        let fpath = f.path();
        let fpath = fpath.to_str().ok_or(Report::msg("invalid path"))?;

        // Name of the file/directory to create in destination
        let dname = f.file_name();
        let dname = dname.to_str().ok_or(Report::msg("invalid file name"))?;
        // Path to the file/directory in the destination
        let opath = dest.to_owned() + "/" + dname;

        // If the source is directory, recursively call itself
        if f.file_type()?.is_dir() {
            make_dir(fpath, &opath, base, conf)?;
            continue;
        }

        // Path relative to the template
        let frel =
            diff_paths(fpath, base).ok_or(Report::msg("Invalid base path"))?;
        let frel = frel.to_str().ok_or(Report::msg("Invalid path"))?;

        if let Some(c) = conf.files.get(frel) {
            // expand the file and its name if it is in config
            make_file_name(c, &conf.vars, fpath, dest, dname)?;
        } else {
            // copy the file if it isn't mentioned in the config
            copy(fpath, opath)?;
        }
    }

    Ok(())
}

/// Expands expression for the filename `dname` in the source directory `src`
/// to the destination directory `dpath`. `info` specifies what to do with the
/// file contents. For expansions uses the variables in `vars`
///
/// #### Used by:
/// `make_dir`
fn make_file_name(
    info: &MakeInfoEnum,
    vars: &HashMap<String, String>,
    src: &str,
    dpath: &str,
    dname: &str,
) -> Result<()> {
    // buffer for custom name
    let mut buf = String::new();
    // Determine the action and file name
    let (action, name) = match info {
        MakeInfoEnum::TypeOnly(a) => (*a, dname),
        MakeInfoEnum::Info(i) => {
            if i.name.len() == 0 {
                (i.action, dname)
            } else {
                // Get the name if the template specifies other than the
                // default
                make_name(
                    &mut CharRW::new(i.name.as_bytes(), [].as_mut()),
                    vars,
                    &mut buf,
                )?;
                // skip files with no name
                if buf.len() == 0 {
                    return Ok(())
                }
                (i.action, buf.as_str())
            }
        }
    };

    // create the file path
    let dest = dpath.to_owned() + "/" + name;

    // do the action with the file
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

/// Evaluates expressions in filename in the read of `rw` and appends the
/// name to `out`. Uses variables in `vars`.
///
/// The difference between `make_name` and `make_file` is that `make_name`
/// outputs to a string and `make_file` outputs to the write of `rw`.
///
/// #### Used by:
/// `make_file_name`
fn make_name<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
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

/// Evaluates expressions in read of `rw` and outputs them to the write of
/// `rw`. Uses the variables in `vars`.
///
/// The difference between `make_name` and `make_file` is that `make_name`
/// outputs to a string and `make_file` outputs to the write of `rw`.
///
/// #### Used by:
/// `make_file_name`
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

/// Evaluates single expression (without the '${') from read of `rw` and
/// outputs it to the write of `rw`. Uses variables in `vars`.
///
/// It wraps the `make_expr_buf` function.
///
/// #### Used by:
/// `make_file`
fn make_expr<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
) -> Result<()> {
    let mut buf = String::new();
    make_expr_buf(rw, vars, &mut buf)?;
    rw.write_str(&buf)
}

// TODO: Unite make_expr and make_expr_buf (maybe edit the CharRW to be able
// to output to a string)

/// Evaluates single expression (without the '${') from read of `rw` and
/// appends it to `out`.
///
/// #### Used by:
/// `make_name`, `make_expr`
fn make_expr_buf<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
    let mut buf = String::new();
    if read_exprs(rw, vars, &mut buf)? {
        return Err(Report::msg(format!("Invalid token '{}'", rw.cur)));
    }
    out.push_str(&buf);
    Ok(())
}

/// Reads expression blocks from read of `rw` and appends them to `out`.
/// Uses variables in `vars`.
///
/// #### Used by:
/// `make_expr_buf`, `read_condition`
fn read_exprs<R: Read, W: Write>(rw: &mut CharRW<R, W>, vars: &HashMap<String, String>, out: &mut String) -> Result<bool> {
    while !matches!(rw.cur, Char('}') | Char(':') | Eof | NoData) {
        read_expr(rw, vars, out)?;
    }

    Ok(matches!(rw.cur, Char(':')))
}

/// Evaluates single expression block in a expression in read of `rw` and
/// appends it to `out`. Uses the variables in `vars`.
///
/// #### Used by:
/// `read_exprs`
fn read_expr<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String,
) -> Result<()> {
    match rw.cur {
        Char('\'') => read_str_literal(rw, out)?,
        Char('?') => read_condition(rw, vars, out)?,
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

/// Skips all whitespace characters in read of `rw`.
///
/// #### Used by:
/// `read_expr`
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

/// Reads string literal from read of `rw` and appends it to `out`.
///
/// #### Used by:
/// `read_expr`
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

/// Reads the escape sequence from read of `rw` and appends it to `out`.
///
/// #### Used by:
/// `read_str_literal`
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
    Ok(())
}

/// Reads variable name from read of `rw` and append its value to `out`.
/// Uses variables in `vars`.
///
/// #### Used by:
/// `read_expr`
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

/// If `out` is empty skips expression blocks until the token ':' and reads the
/// following blocks into `out`. Otherwise reads the following blocks and skips
/// the next.
///
/// #### Used by:
/// `read_expr`
fn read_condition<R: Read, W: Write>(
    rw: &mut CharRW<R, W>,
    vars: &HashMap<String, String>,
    out: &mut String
) -> Result<()> {
    rw.read()?;
    if out.len() == 0 {
        read_exprs(rw, vars, out)?;
        if !matches!(rw.cur, Char(':')) {
            return Err(Report::msg("Expected ':'"));
        }
        rw.read()?;
        out.clear();
        read_exprs(rw, vars, out)?;
    } else {
        out.clear();
        read_exprs(rw, vars, out)?;
        if !matches!(rw.cur, Char(':')) {
            return Err(Report::msg("Expected ':'"));
        }
        rw.read()?;
        let mut buf = String::new();
        // skip the second part
        read_exprs(rw, vars, &mut buf)?;
    }
    Ok(())
}

/// Recursively copies the directory `from` to the directory `to`.
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
