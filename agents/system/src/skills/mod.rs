use crate::prelude::*;

/// The agent skill name
#[derive(Clone, Copy, Debug, Display, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[display(rename = "snake_case")]
pub enum SkillName {
    SystemInfo,
    MediaControl,
    PowerManagement,
    ThemeSwitching,
}

/// The agent skill info
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Skill {
    pub name: SkillName,
    pub description: &'static str,
    #[serde(skip)]
    pub tools_list: fn() -> Vec<anylm::Tool>,
}

impl Skill {
    pub const fn new(
        name: SkillName,
        description: &'static str,
        tools_list: fn() -> Vec<anylm::Tool>,
    ) -> Self {
        Self {
            name,
            description,
            tools_list,
        }
    }

    pub fn tools_list(&self) -> Vec<anylm::Tool> {
        (self.tools_list)()
    }
}
