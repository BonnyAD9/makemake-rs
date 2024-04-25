use std::{borrow::Cow, collections::HashMap, fs::File, path::Path};

use serde::{Deserialize, Serialize};

use crate::err::Result;

#[derive(Serialize, Deserialize)]
pub struct Alias {
    pub template: String,
    pub vars: HashMap<Cow<'static, str>, Cow<'static, str>>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub vars: HashMap<Cow<'static, str>, Cow<'static, str>>,
    pub aliases: HashMap<String, Alias>,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    pub fn to_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        serde_json::to_writer_pretty(File::create(path)?, &self)?;
        Ok(())
    }
}
