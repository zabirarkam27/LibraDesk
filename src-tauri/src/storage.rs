use std::{
    fs, io,
    path::{Path, PathBuf},
};

const DIRECTORIES: [&str; 6] = ["database", "thumbnails", "cache", "backups", "logs", "temp"];

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn prepare_directory(path: &Path) -> io::Result<()> {
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || is_link(&metadata) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected a local directory",
        ));
    }
    tempfile::NamedTempFile::new_in(path)?.close()
}

pub fn initialize(root: &Path) -> io::Result<PathBuf> {
    prepare_directory(root)?;
    for directory in DIRECTORIES {
        prepare_directory(&root.join(directory))?;
    }
    let database_dir = root.join("database");
    for file in [
        "library.sqlite3",
        "library.sqlite3-wal",
        "library.sqlite3-shm",
        "library.sqlite3-journal",
    ] {
        match fs::symlink_metadata(database_dir.join(file)) {
            Ok(metadata) if metadata.is_file() && !is_link(&metadata) => {}
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected a local database file",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(database_dir.join("library.sqlite3"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_directories_twice_and_preserves_existing_files() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path().join("LibraDesk");
        let database = initialize(&root).unwrap();
        let sentinel = root.join("backups").join("existing");
        fs::write(&sentinel, "keep").unwrap();
        assert_eq!(initialize(&root).unwrap(), database);
        assert_eq!(fs::read_to_string(&sentinel).unwrap(), "keep");
        for directory in DIRECTORIES {
            assert!(root.join(directory).is_dir());
        }
        assert!(fs::read_dir(root.join("temp")).unwrap().next().is_none());
    }

    #[test]
    fn rejects_a_file_in_place_of_a_directory_and_a_directory_in_place_of_the_db() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path().join("LibraDesk");
        fs::write(&root, "preserve").unwrap();
        assert_eq!(
            initialize(&root).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(fs::read_to_string(&root).unwrap(), "preserve");
        fs::remove_file(&root).unwrap();
        let database = initialize(&root).unwrap();
        fs::create_dir(database).unwrap();
        assert_eq!(
            initialize(&root).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
