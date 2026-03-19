use crate::prelude::*;
use anylm::{Schema, SchemaKind};

/// The agent settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestAgent {
    pub enable: bool,
    pub name: String,
    pub description: String,
    pub debug_exec: PathBuf,
    pub exec: PathBuf,
    pub is_server: bool,
}

/// The agent using example struct
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestExample {
    pub query: String,
    pub data: HashMap<String, JsonValue>,
}

/// The agent handler options
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestAction {
    pub enable: bool,
    pub description: String,
    pub arguments: HashMap<String, ManifestArgument>,
    pub examples: Vec<ManifestExample>,
}

/// The agent argument structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestArgument {
    /// The schema type
    #[serde(rename = "type")]
    pub kind: SchemaKind,
    /// The schema description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The string value variants
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub variants: Option<Vec<String>>,
    /// The minimum value for number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// The maximum value for number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// The array items type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    /// The object properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<ManifestArgument>>>,
    /// The required object properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(default)]
    pub optional: bool,
}

/// The agent manifest structure
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub agent: ManifestAgent,
    pub actions: HashMap<String, ManifestAction>,
}
