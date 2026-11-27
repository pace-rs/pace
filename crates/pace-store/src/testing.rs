//! A scratch directory for tests, where the `files` backend's syncs cost
//! nothing.

use std::{io, path::Path};

use tempfile::TempDir;

/// A temporary directory in memory where the system has one (`/dev/shm` on
/// Linux), else in the system's temporary folder.
///
/// The `files` backend syncs each file it writes. On a disk under load one
/// sync costs tens of milliseconds, and a journey test of eight steps then
/// takes longer than codegate's slow-tests threshold of 0.5 s; in memory the
/// sync costs nothing.
///
/// # Errors
///
/// The directory cannot be created.
pub fn scratch_dir() -> io::Result<TempDir> {
    let memory = Path::new("/dev/shm");
    if memory.is_dir() {
        tempfile::tempdir_in(memory)
    } else {
        tempfile::tempdir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_dir_lies_in_memory_where_the_system_has_it() -> io::Result<()> {
        let dir = scratch_dir()?;
        let memory = Path::new("/dev/shm");
        assert_eq!(dir.path().starts_with(memory), memory.is_dir());
        Ok(())
    }
}
