use eyre::{Report, Result};
use std::fs::{copy, create_dir_all, read_dir};

// just a directory copy rn
// when this is updated, update also load_template
pub fn create_template(src: &str, out: &str) -> Result<()> {
    create_dir_all(out)?;

    for f in read_dir(src)? {
        let f = f?;

        let fpath = f.path();
        let fpath = fpath.to_str().ok_or(Report::msg("invalid path"))?;

        let opath = out.to_owned()
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

pub fn load_tempalte(src: &str, dest: &str) -> Result<()> {
    create_template(src, dest)
}
