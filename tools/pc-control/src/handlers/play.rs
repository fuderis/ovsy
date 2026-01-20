use crate::prelude::*;
use tokio::process::Command;
use tokio::fs as tfs;
use std::fs;
use std::process::Stdio;
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
    match create_playlist(&playlist_dir).await {
        Ok(playlist_file) => {
            info!("🎵 Play music '{}'.", playlist_dir.to_string_lossy().replace("\\", "/"));
            
            #[cfg(unix)]
            {
                // close audacious processes:
                let _ = close_audacious().await;

                // open playlist file:
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&fmt!("setsid xdg-open '{}' > /dev/null 2>&1 &", playlist_file.display()))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }

            #[cfg(windows)]
            {
                let _ = Command::new("cmd")
                    .args(["/C", "start", "", &str!(playlist_file.to_string_lossy())])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
               }
        }
        Err(e) => {
            err!("{e}");
            return Json(json!({ "status": 500, "error": fmt!("{e}") }));
        }
    }
    
    Json(json!({ "status": 200 }))
}

/// Closes Audacious processes
#[cfg(unix)]
async fn close_audacious() -> Result<()> {
    // find audacious process by port:
    let output = Command::new("pgrep")
        .arg("audacious")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await?;

    if !output.status.success() { return Ok(()); }

    // parse audacious PIDs: 
    let pids: Vec<i32> = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split('\n')
        // exclude system ones:
        .filter_map(|line| line.trim().parse().ok().filter(|&pid| pid > 1))
        .collect();

    if pids.is_empty() { return Ok(()); }
    info!("Found {count} audacious processes: {pids:?}", count = pids.len());

    // stop all processes: 
    for &pid in &pids {
        let pid_str = pid.to_string();
        if Command::new("kill").args(["-TERM", &pid_str]).status().await?.success() {
            info!("Graceful stop PID {pid}");
        } else {
            let _ = Command::new("kill").args(["-9", &pid_str]).status();
            info!("Force kill PID {pid}");
        }
    }

    Ok(())
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

/// Create playlist file
async fn create_playlist<P: AsRef<Path>>(dir: P) -> Result<PathBuf> {
    let playlist_path = dir.as_ref().join("playlist.m3u");
        
    let songs_list = read_song_files(dir)?;
    let mut content = Vec::new();
    content.extend_from_slice(b"#EXTM3U\n");
    
    for song in &songs_list {
        let unix_path = song.replace('\\', "/");
        let filename = std::path::Path::new(&unix_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(['—', '–'], "-"); // Audacious не любит em-dash
    
        content.extend_from_slice(format!("#EXTINF:-1,{}\n", filename).as_bytes());
        content.extend_from_slice(unix_path.as_bytes());
        content.extend_from_slice(b"\n");
    }
                                               
    tfs::write(&playlist_path, content)
        .await
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
