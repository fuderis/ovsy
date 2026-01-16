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
    pub descr: String,
    pub args: HashMap<String, ManifestToolArgument>,
}

/// The manifest tool argument structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestToolArgument {
    pub format: String,
    pub variants: Option<Vec<String>>,
    pub optional: bool,
    pub example: String,
}

/// The manifest data structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub tool: ManifestTool,
    pub server: Option<ManifestServer>,
    pub actions: HashMap<String, ManifestToolAction>,
}
