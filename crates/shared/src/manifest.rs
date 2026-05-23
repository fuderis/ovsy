use anylm::Tool;
use serde::{Deserialize, Serialize};

/// The AI agent options
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentOptions {
    pub name: String,
    pub description: String,
    pub prompt: String,
}

/// The AI agent manifest
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub agent: AgentOptions,
    pub tools: Vec<Tool>,
}

/* EXAMPLE:
[agent]
name = "music-agent"
description = "Search/Play music by genre, artist/band, album or song name"
prompt = ""

[[tools]]
name = "search"
description = "Searches for music by genre, artist/band name, album, or specific song"
parameters = [
    { type = "string", name = "genre", description = "The music genre", optional = true },
    { type = "string", name = "artist", description = "The music artist/band name", optional = true },
    { type = "string", name = "album", description = "The music album name (requires artist/band name parameter)", optional = true },
    { type = "string", name = "song", description = "The certain song name", optional = true }
]

[[tools]]
name = "play"
description = "Plays music from the specified directory"
parameters = [
    { type = "string", name = "path", description = "Path to the music directory", optional = false }
]
*/
