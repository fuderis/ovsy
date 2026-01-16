use crate::prelude::*;
use tokio::process::Command;
use std::fs;
// use tokio::fs as tfs;

const SEARCH_COEF: f32 = 0.5;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    author: String,
    album: Option<String>,
}

/// Api '/play' handler
pub async fn handle(Json(data): Json<QueryData>) -> Json<JsonValue> {
    let dirs = Settings::get().music.dirs.clone();

    // search playlist path:
    info!("🔎 Search for playlist '{}'..", &data.author);
    let playlist_dir = match search_playlist(dirs, data.author, data.album).await {
        Ok(r) => r,
        Err(e) => {
            err!("{e}");
            return Json(json!({ "status": 500, "error": fmt!("{e}") }));
        }
    };

    // create playlist.m3u file:
    match create_playlist(&playlist_dir) {
        Ok(playlist_file) => {
            info!("🎵 Play music '{}'.", playlist_dir.to_string_lossy().replace("\\", "/"));
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", "start", "", &str!(playlist_file.to_string_lossy())]);
            cmd.creation_flags(0x08000000);

            if let Err(e) = cmd.spawn() {
                err!("{e}");
                return Json(json!({ "status": 500, "error": fmt!("{e}") }));
            };
        }
        Err(e) => {
            err!("{e}");
            return Json(json!({ "status": 500, "error": fmt!("{e}") }));
        }
    }
    
    Json(json!({ "status": 200 }))
}

/// Searches playlists in music folders
async fn search_playlist(dirs: Vec<PathBuf>, author: String, album: Option<String>) -> Result<PathBuf> {
    // search in dirs:
    let results = scan_dirs(&dirs, &author, SEARCH_COEF, true).await?;
    let best = results.get(0)
        .map(|p| p.to_owned())
        .ok_or(Error::PlaylistNotFound(author))?;

    // search in subdirs:
    let best = if let Some(album) = album {
        let results = scan_dirs(&[&best], &album, SEARCH_COEF, true).await?;
        results.get(0)
            .map(|p| p.to_owned())
            .ok_or(Error::AlbumNotFound(album, str!(best.to_string_lossy())))?
    } else {
        best
    };

    Ok(best)
}

/// Create playlist & play it
fn create_playlist<P: AsRef<Path>>(dir: P) -> Result<PathBuf> {
    let playlist_path = dir.as_ref().join("playlist.m3u");
    if playlist_path.exists() { return Ok(playlist_path); }
    let songs_list = read_song_files(dir)?;

    fs::write(&playlist_path, songs_list.join("\n"))
        .map_err(|e| fmt!("Failed to create playlist: {e}"))?;

    Ok(playlist_path)
}

/// Reads song files in dir
fn read_song_files<P: AsRef<Path>>(dir: P) -> Result<Vec<String>> {
    let exts = ["mp3", "flac", "wav"];
    let mut songs = vec![];
    
    for entry in fs::read_dir(dir).map_err(|e| fmt!("Failed to read playlist folder: {e}"))? {
        let path = entry?.path();

        if path.is_dir() {
            songs.extend(read_song_files(path)?);
        } else if path.is_file() && path.extension().map_or(false, |ext| exts.contains(&ext.to_str().unwrap_or(""))) {
            songs.push(path.to_string_lossy().to_string());
        }
    }

    Ok(songs)
}


/// Scans dirs and searches folders by Levenshtaine distance
async fn scan_dirs<P>(dirs: &[P], search: &str, min_coef: f32, only_folders: bool) -> Result<Vec<PathBuf>>
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
    let matches: Vec<_> = fuzzy_cmp::search_filter(&entries, search, min_coef, true, |s| s.file_name().map(|s| s.to_str().unwrap_or("")).unwrap_or(""));
    let results: Vec<PathBuf> = matches.iter()
        .map(|(_, entry)| entry.clone())
        .collect();

    Ok(results)
}
