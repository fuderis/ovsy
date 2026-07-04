use crate::prelude::*;
use music_index::{MusicIndexer, SearchIntent};

static MUSIC_INDEX: State<Option<MusicIndexer>> = State::default();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicAction {
    pub query: Option<String>,
    pub band: Option<String>,
    pub album: Option<String>,
    pub track: Option<String>,
    pub genre: Option<String>,
}

async fn music_index() -> Result<MusicIndexer> {
    if MUSIC_INDEX.get().await.is_none() {
        let index = MusicIndexer::scan_default(app_data().join("db/music.cache")).await?;
        MUSIC_INDEX.set(Some(index)).await;
    }

    MUSIC_INDEX
        .dirty_get()
        .as_ref()
        .clone()
        .ok_or_else(|| str!("Failed to initialize music indexer").into())
}

/// API: Handles the music searching
#[log(skip_all, fields(action))]
pub async fn handle_search_music(tx: Sender<Bytes>, action: MusicAction) -> Result<()> {
    let music_index = music_index().await?;

    let intent = if let Some(query) = action.query {
        SearchIntent::Global(query)
    } else {
        SearchIntent::Targeted {
            band: action.band,
            album: action.album,
            track: action.track,
            genre: action.genre,
        }
    };

    let target = music_index.search(intent);
    let tracks = target.tracks();

    let msg = if tracks.is_empty() {
        str!("No matching music was found.")
    } else {
        str!("Found {count} matching track(s).", count = tracks.len())
    };

    info!("{msg}");
    tx.send(Chunk::answer(msg)).await?;

    Ok(())
}

/// Handles the music playback
#[log(skip_all, fields(action))]
pub async fn handle_play_music(tx: Sender<Bytes>, action: MusicAction) -> Result<()> {
    let music_index = music_index().await?;

    let intent = if let Some(query) = action.query {
        SearchIntent::Global(query)
    } else {
        SearchIntent::Targeted {
            band: action.band,
            album: action.album,
            track: action.track,
            genre: action.genre,
        }
    };

    let target = music_index.search(intent);
    let tracks = target.tracks();

    if tracks.is_empty() {
        let msg = str!("No matching music was found.");
        info!("{msg}");
        tx.send(Chunk::answer(msg)).await?;
        return Ok(());
    }

    music_index
        .play(target, app_data().join("db/playlist.m3u"))
        .await?;

    let msg = str!(
        "Started playback of {count} track(s).",
        count = tracks.len()
    );

    info!("{msg}");
    tx.send(Chunk::answer(msg)).await?;

    Ok(())
}
