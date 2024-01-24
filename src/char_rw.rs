use eyre::{Report, Result};
use std::io::{Read, Write};
use utf8_read::{
    Char::{Char, NoData},
    Reader,
};

/// Struct for reading and writing chars from Read and Write traits
/// Remembers the last readed char
pub struct CharRW<R: Read, W: Write> {
    reader: Reader<R>,
    writer: W,
    /// the last char readed by any of the read methods
    pub cur: utf8_read::Char,
}

impl<R: Read, W: Write> CharRW<R, W> {
    /// Writes single char `c` to the writer
    pub fn write(&mut self, c: char) -> Result<()> {
        let b = c.to_string();
        self.write_bytes(b.as_bytes())
    }

    /// Writes string `c` to the writer
    pub fn write_str(&mut self, c: &str) -> Result<()> {
        self.write_bytes(c.as_bytes())
    }

    /// Writes byte array `b` to the writer
    pub fn write_bytes(&mut self, b: &[u8]) -> Result<()> {
        if self.writer.write(b)? != b.len() {
            Err(Report::msg("cannot write"))
        } else {
            Ok(())
        }
    }

    /// Reads single char from the reader and stores it in the .cur field.
    /// The readed char is also returned.
    pub fn read(&mut self) -> Result<utf8_read::Char> {
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
    pub fn new(r: R, w: W) -> Self {
        CharRW {
            reader: Reader::new(r),
            writer: w,
            cur: NoData,
        }
    }

    /// Appends chars to `out` while `p` returns `true`.
    /// The first unreaded char is stored in the .cur field.
    pub fn read_while<P: Fn(char) -> bool>(
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
