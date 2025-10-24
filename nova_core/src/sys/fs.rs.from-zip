use std::fs::{self, File, Metadata};
use std::io::Write;
use std::path::Path;

use anyhow::Result;

/// Reads an entire file into a string.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

/// Writes data to disk, creating parent directories if necessary.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(contents.as_ref())?;
    Ok(())
}

/// Reads raw bytes from disk.
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

/// Returns metadata about the file.
pub fn stat(path: impl AsRef<Path>) -> Result<Metadata> {
    Ok(fs::metadata(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn roundtrip_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("nova_core_fs.txt");
        write(&file, "hello").unwrap();
        assert_eq!(read_to_string(&file).unwrap(), "hello");
        let metadata = stat(&file).unwrap();
        assert!(metadata.is_file());
        fs::remove_file(&file).unwrap();
    }
}
