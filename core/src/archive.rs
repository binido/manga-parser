use crate::error::{Error, Result};
use crate::natural_sort;
use std::fs::File;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// KCC читает эти форматы напрямую, поэтому перекодировать ничего не нужно —
/// достаточно не потерять их при сборке.
const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "gif", "webp"];

pub fn is_zip(path: &Path) -> bool {
    has_extension(path, &["zip"])
}

pub fn find_archives(root: &Path) -> Vec<PathBuf> {
    find_files(root, is_zip)
}

pub fn find_images(root: &Path) -> Vec<PathBuf> {
    find_files(root, |path| has_extension(path, &IMAGE_EXTENSIONS))
}

/// Распаковывает архив целиком. Записи с путями, ведущими наружу целевой
/// папки, `zip` отбрасывает сам.
pub fn extract(archive: &Path, into: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|source| Error::Archive {
        path: archive.to_owned(),
        source,
    })?;

    zip.extract(into).map_err(|source| Error::Archive {
        path: archive.to_owned(),
        source,
    })
}

fn find_files(root: &Path, accept: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| accept(path))
        .collect();

    files.sort_by(|a, b| natural_sort::compare_paths(a, b));
    files
}

fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            allowed.contains(&ext.as_str())
        })
}
