use crate::prelude::*;
use std::process::Stdio;
use tokio::{fs, process::Command};

/// Levenshtein distance coefficient
const SEARCH_COEF: f32 = 0.5;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    author: String,
    album: Option<String>,
}

/// Api '/play' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let dirs = Settings::get().music.dirs.clone();

    // close audacious processes:
    #[cfg(unix)]
    {
        close_audacious().await.ok();
    }

    // search playlist path:
    info!("Search for playlist '{}'..", &data.author);
    let playlist_dir = match search_playlist(dirs, data.author, data.album).await {
        Ok(r) => r,
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // create playlist.m3u file:
    match create_playlist(&playlist_dir).await {
        Ok(playlist_file) => {
            let play_dir = playlist_dir.to_string_lossy().replace("\\", "/");
            info!("Trying to play music on {play_dir}..");

            // open playlist file:
            let status = {
                #[cfg(unix)]
                {
                    Command::new("sh")
                        .arg("-c")
                        .arg(fmt!(
                            "setsid xdg-open '{}' > /dev/null 2>&1 &",
                            playlist_file.display()
                        ))
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                }
                #[cfg(windows)]
                {
                    Command::new("cmd")
                        .args(["/C", "start", "", &str!(playlist_file.to_string_lossy())])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                }
            };

            match status {
                Ok(_) => {
                    info!("Playing music on {play_dir}");
                    (
                        StatusCode::OK,
                        HeaderMap::from_iter(map! {
                            header::CONTENT_TYPE => "text/plain".parse().unwrap()
                        }),
                        Body::new(fmt!("Playing music on {play_dir}")),
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("{e}");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                }
            }
        }
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
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

    if !output.status.success() {
        return Ok(());
    }

    // parse audacious PIDs:
    let pids: Vec<i32> = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split('\n')
        // exclude system ones:
        .filter_map(|line| line.trim().parse().ok().filter(|&pid| pid > 1))
        .collect();

    if pids.is_empty() {
        return Ok(());
    }
    let count = pids.len();
    info!("Found {count} audacious processes: {pids:?}",);

    // stop all processes:
    for &pid in &pids {
        let pid_str = pid.to_string();
        if Command::new("kill")
            .args(["-TERM", &pid_str])
            .status()
            .await?
            .success()
        {
            info!("Graceful stop PID {pid}");
        } else {
            let _ = Command::new("kill").args(["-9", &pid_str]).status().await;
            info!("Force kill PID {pid}");
        }
    }

    // wait for close audacious:
    if count > 0 {
        sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

/// Searches playlists in music folders
async fn search_playlist(
    dirs: Vec<PathBuf>,
    author: String,
    album: Option<String>,
) -> Result<PathBuf> {
    // search in dirs:
    let results = utils::scan_dirs(&dirs, &author, SEARCH_COEF, true).await?;
    let best = results
        .first()
        .map(|p| p.to_owned())
        .ok_or(Error::PlaylistNotFound(author))?;

    // search in subdirs:
    let best = if let Some(album) = album {
        let results = utils::scan_dirs(&[&best], &album, SEARCH_COEF, true).await?;
        results
            .first()
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

    let songs_list = read_song_files(dir).await?;
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

    fs::write(&playlist_path, content)
        .await
        .map_err(|e| fmt!("Failed to create playlist: {e}"))?;

    Ok(playlist_path)
}

/// Reads song files in dir
async fn read_song_files<P: AsRef<Path>>(dir: P) -> Result<Vec<String>> {
    let mut songs = Vec::new();
    let mut stack = vec![fs::read_dir(dir).await?];

    while let Some(mut entries) = stack.pop() {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                stack.push(fs::read_dir(path).await?);
            } else if is_music_file(&path) {
                songs.push(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(songs)
}

/// Checks file extension for song format
fn is_music_file<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    const EXTS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a"];
    path.as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| EXTS.contains(&ext.to_lowercase().as_str()))
}
