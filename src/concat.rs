use std::fs::File;
use std::io;
use std::path::Path;

pub fn merge_mp3(paths: &[std::path::PathBuf], output: &Path) -> io::Result<()> {
    let mut out = File::create(output)?;
    for p in paths {
        let metadata = std::fs::metadata(p)?;
        if metadata.len() > 0 {
            let mut input = File::open(p)?;
            io::copy(&mut input, &mut out)?;
        }
    }
    Ok(())
}
