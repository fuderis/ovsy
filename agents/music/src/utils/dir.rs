use crate::prelude::*;
use tokio::fs;

/// Returns subdirs
pub async fn flatten_subdirs<P>(dirs: &[P]) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut subdirs = vec![];

    for dir in dirs {
        let mut entries = fs::read_dir(dir.as_ref()).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_dir() {
                subdirs.push(entry.path().to_path_buf())
            }
        }
    }

    Ok(subdirs)
}

/// Scans next dirs level
pub async fn smart_scan<P>(
    levels: &[Vec<P>],
    pattern: &str,
    min_coef: f32,
    only_folders: bool,
) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut results = vec![];

    for dirs in levels.iter().rev() {
        let found = scan_dirs(dirs, pattern, min_coef, only_folders).await?;
        if !found.is_empty() {
            results.extend(found);
        }
    }

    Ok(results)
}

/// Scans dirs and searches folders by Levenshtaine distance
pub async fn scan_dirs<P>(
    dirs: &[P],
    pattern: &str,
    min_coef: f32,
    only_folders: bool,
) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    // read dir entries:
    let mut entries = vec![];

    for dir in dirs {
        let mut subdirs = fs::read_dir(dir.as_ref()).await?;
        while let Ok(Some(entry)) = subdirs.next_entry().await {
            if !only_folders || entry.path().is_dir() {
                entries.push(entry.path().to_path_buf())
            }
        }
    }

    // matching results:
    let matches: Vec<_> = fuzzy_cmp::search_filter(&entries, pattern, min_coef, true, |s| {
        s.file_name()
            .map(|s| s.to_str().unwrap_or(""))
            .unwrap_or("")
    });
    let results: Vec<PathBuf> = matches.iter().map(|(_, entry)| entry.clone()).collect();

    Ok(results)
}
