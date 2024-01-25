use std::{fmt, io};

pub struct ToFmtWrite<T>(pub T) where T: io::Write;

impl<T> fmt::Write for ToFmtWrite<T> where T: io::Write {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

pub struct FakeWriter;

impl fmt::Write for FakeWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Ok(())
    }
}
