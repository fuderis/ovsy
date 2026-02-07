use crate::prelude::*;
use std::process::Stdio;
use tokio::{fs, process::Command};

/// Levenshtein distance coefficient
const SEARCH_COEF: f32 = 0.5;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    genre: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    song: Option<String>,
    #[serde(default)]
    search: bool,
}

/// Api '/music' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let search = data.search;
    let name = [&data.genre, &data.artist, &data.album, &data.song]
        .iter()
        .filter_map(|&opt| opt.clone())
        .collect::<Vec<_>>()
        .join(" / ");

    // search playlist path:
    info!("Search for music '{name}'..");
    let playlists = match search_playlists(data, name).await {
        Ok(r) => {
            info!("Found music: {r:?}");
            r
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, fmt!("{e}")).into_response();
        }
    };

    // return results (if need):
    if search {
        return (
            StatusCode::OK,
            HeaderMap::from_iter(map! {
                header::CONTENT_TYPE => "text/plain".parse().unwrap()
            }),
            Body::new(fmt!(
                "Found music: {}",
                json::to_string_pretty(&playlists).unwrap()
            )),
        )
            .into_response();
    }

    // close audacious processes:
    #[cfg(unix)]
    {
        close_audacious().await.ok();
    }

    // create playlist file & run it:
    match create_playlist(&playlists, path!("$/playlist.m3u")).await {
        Ok(playlist_file) => {
            info!("Trying to play music..");

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
                    info!("Play music success");
                    (
                        StatusCode::OK,
                        HeaderMap::from_iter(map! {
                            header::CONTENT_TYPE => "text/plain".parse().unwrap()
                        }),
                        Body::new(fmt!("Playing music dirs: {playlists:#?}")),
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

/// Smart search playlists
async fn search_playlists(data: QueryData, name: String) -> Result<Vec<PathBuf>> {
    let mut search_dirs = vec![Settings::get().music.dirs.clone()];
    let stages = [&data.genre, &data.artist, &data.album];

    // search folders:
    for param in stages {
        if let Some(pattern) = param {
            let results = utils::smart_scan(&search_dirs, pattern, SEARCH_COEF, true).await?;
            search_dirs = search_dirs[1..].to_vec();

            if !results.is_empty() {
                search_dirs.push(results);
            } else {
                return Err(Error::PlaylistNotFound(name).into());
            }
        } else {
            let new_level = utils::flatten_subdirs(search_dirs.last().unwrap())
                .await
                .unwrap_or_default();
            search_dirs.push(new_level);
        }
    }

    // search song:
    let playlists = if let Some(song) = &data.song {
        let results = utils::smart_scan(&search_dirs, song, SEARCH_COEF, false).await?;
        if !results.is_empty() {
            results
        } else {
            return Err(Error::PlaylistNotFound(name).into());
        }
    } else if data.genre.is_some() && data.artist.is_none() {
        search_dirs.get(search_dirs.len() - 2).unwrap().clone()
    } else {
        vec![search_dirs.get(search_dirs.len() - 2).unwrap()[0].clone()]
    };

    Ok(playlists)
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

/// Create playlist file
async fn create_playlist<P, P2>(dirs: &[P], file: P2) -> Result<PathBuf>
where
    P: AsRef<Path>,
    P2: Into<PathBuf>,
{
    let playlist_path = file.into();

    // read song files:
    let mut songs_list = vec![];
    for dir in dirs {
        songs_list.extend(read_song_files(dir).await?);
    }

    let mut content = Vec::new();
    content.extend_from_slice(b"#EXTM3U\n");

    // add songs to playlist:
    for song in &songs_list {
        let unix_path = song.replace('\\', "/");
        let filename = std::path::Path::new(&unix_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

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
    let dir = dir.as_ref();
    if dir.is_file() {
        return Ok(vec![str!(dir.to_string_lossy())]);
    }

    let mut songs = Vec::new();
    let mut stack = vec![fs::read_dir(dir).await?];

    while let Some(mut entries) = stack.pop() {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                stack.push(fs::read_dir(path).await?);
            } else if is_music_file(&path) {
                songs.push(str!(path.to_string_lossy()));
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
