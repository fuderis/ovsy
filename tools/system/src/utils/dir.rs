use crate::prelude::*;
use std::fs;

/// Scans dirs and searches folders by Levenshtaine distance
pub async fn scan_dirs<P>(
    dirs: &[P],
    search: &str,
    min_coef: f32,
    only_folders: bool,
) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    // read dir entries:
    let mut entries = vec![];

    for dir in dirs {
        for entry in fs::read_dir(dir.as_ref())? {
            let entry = entry?;

            if !only_folders || entry.path().is_dir() {
                entries.push(entry.path().to_path_buf())
            }
        }
    }

    // matching results:
    let matches: Vec<_> = fuzzy_cmp::search_filter(&entries, search, min_coef, true, |s| {
        s.file_name()
            .map(|s| s.to_str().unwrap_or(""))
            .unwrap_or("")
    });
    let results: Vec<PathBuf> = matches.iter().map(|(_, entry)| entry.clone()).collect();

    Ok(results)
}
