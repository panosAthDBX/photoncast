use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickLinksConfig {
    pub enabled: bool,
}

impl Default for QuickLinksConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
