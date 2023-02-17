use eyre::{Report, Result};
use std::fs::{copy, create_dir_all, read_dir};

pub fn create_template(src: &str, out: &str) -> Result<()> {
    copy_dir(src, out)
}

pub fn load_tempalte(src: &str, dest: &str) -> Result<()> {
    copy_dir(src, dest)
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
            create_template(fpath, &opath)?;
            continue;
        }

        copy(fpath, opath)?;
    }

    Ok(())
}
