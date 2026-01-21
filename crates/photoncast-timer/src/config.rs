use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimerConfig {
    pub enabled: bool,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
