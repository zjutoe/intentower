//! Reject report writes that alias inputs or the other report artifact.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

fn absent(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

/// Resolve existing prefixes before handling `..`, including a return from
/// missing directories to an existing prefix. Missing targets are compared
/// prospectively; their writability is still handled by the existing writer.
fn prospective_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other.as_os_str()),
        }
        match fs::canonicalize(&resolved) {
            Ok(existing) => resolved = existing,
            Err(error) if absent(&error) => {
                match fs::symlink_metadata(&resolved) {
                    // An existing entry that cannot be resolved, including
                    // a dangling symlink, must not be guessed safe.
                    Ok(_) => return Err(error),
                    Err(missing) if absent(&missing) => {}
                    Err(other) => return Err(other),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(resolved)
}

struct Target {
    path: PathBuf,
    file_id: Option<(u64, u64)>,
}

impl Target {
    fn of(path: &Path) -> Result<Self, String> {
        let resolved = prospective_path(path)
            .map_err(|error| format!("cannot resolve output/input path {path:?}: {error}"))?;
        let file_id = match fs::metadata(&resolved) {
            Ok(metadata) => Some((metadata.dev(), metadata.ino())),
            Err(error) if absent(&error) => None,
            Err(error) => {
                return Err(format!(
                    "cannot inspect output/input path {path:?}: {error}"
                ));
            }
        };
        Ok(Self {
            path: resolved,
            file_id,
        })
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.path == other.path || (self.file_id.is_some() && self.file_id == other.file_id)
    }
}

pub fn check(outputs: &[&Path], inputs: &[PathBuf]) -> Result<(), String> {
    let mut protected = inputs
        .iter()
        .map(|path| Ok((path.as_path(), Target::of(path)?)))
        .collect::<Result<Vec<_>, String>>()?;
    for output in outputs {
        let target = Target::of(output)?;
        for (other_path, other) in &protected {
            if target.overlaps(other) {
                return Err(format!("output path {output:?} overlaps {other_path:?}"));
            }
        }
        // Keep the verification report and preflight diagnostic distinct.
        protected.push((output, target));
    }
    Ok(())
}
