use crate::prelude::*;

/// The manifest tool settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestTool {
    pub enable: bool,
    pub name: String,
    pub exec: PathBuf,
}

/// The manifest server configuration
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestServer {
    pub port: u16,
}

/// The manifest tool handler options
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestToolAction {
    pub enable: bool,
    pub description: String,
    pub arguments: HashMap<String, ManifestToolArgument>,
    pub examples: Vec<(String, HashMap<String, JsonValue>)>,
}

/// The manifest tool argument structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestToolArgument {
    pub format: String,
    pub variants: Option<Vec<String>>,
    pub optional: bool,
}

/// The manifest data structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub tool: ManifestTool,
    pub server: Option<ManifestServer>,
    pub actions: HashMap<String, ManifestToolAction>,
}
