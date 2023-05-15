use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};

pub mod keygen;
pub mod node;
pub mod sign;
pub mod smc;

pub fn timestamp(h: i64) -> anyhow::Result<()> {
    let utc_datetime: DateTime<Utc> = Utc::now();
    println!("{} <= UTC", utc_datetime.to_rfc3339());

    let local_datetime: DateTime<Local> = Local::now();
    println!("{} <= Local", local_datetime.to_rfc3339());
    // println!("custom format: {}", local_datetime.format("%a %b %e %T %Y"));

    println!("current timestamp: {:?}", utc_datetime.timestamp_millis());

    if h < 0 {
        anyhow::bail!("Invalid hour");
    }
    let s: i64 = utc_datetime.timestamp_millis() + h * 60 * 60 * 1000;
    println!("{h:?} hour later: {s:?}");

    Ok(())
}

pub fn traverse(path: &Path, cb: &mut dyn FnMut(PathBuf)) -> anyhow::Result<()> {
    for e in read_dir(path)? {
        let e = e?;
        let path = e.path();
        if path.is_dir() {
            traverse(&path, cb)?;
        } else if path.is_file() {
            cb(path);
        }
    }
    Ok(())
}
