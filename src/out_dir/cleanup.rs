use crate::{AllieError, Result};
use std::fs;
use std::path::Path;

/// Remove stale contents while preserving the validated root ownership marker.
/// Nested files with the same name are ordinary stale output. File types come from
/// `DirEntry::file_type`, which does not traverse symlinks: a symlink is
/// removed as a leaf via `remove_file`, never recursed into, so nothing
/// outside the walk root can be touched.
pub(super) fn clean_dir_contents(dir: &Path) -> Result<()> {
    clean_dir_except_manifest(dir, &dir.join(super::MANIFEST_FILE_NAME), &mut remove_entry)
}

pub(super) fn clean_dir_except_manifest(
    dir: &Path,
    manifest_path: &Path,
    remove: &mut impl FnMut(&Path, bool) -> Result<()>,
) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| AllieError::Io {
        context: format!("read directory {}", dir.display()),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AllieError::Io {
            context: format!("read directory entry under {}", dir.display()),
            source,
        })?;
        let path = entry.path();
        if path == manifest_path {
            continue;
        }
        let file_type = entry.file_type().map_err(|source| AllieError::Io {
            context: format!("read file type of {}", path.display()),
            source,
        })?;
        if file_type.is_dir() {
            clean_dir_except_manifest(&path, manifest_path, remove)?;
        }
        remove(&path, file_type.is_dir())?;
    }
    Ok(())
}

fn remove_entry(path: &Path, is_dir: bool) -> Result<()> {
    let result = if is_dir {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    result.map_err(|source| AllieError::Io {
        context: format!(
            "remove stale {} {}",
            if is_dir { "directory" } else { "file" },
            path.display()
        ),
        source,
    })
}
