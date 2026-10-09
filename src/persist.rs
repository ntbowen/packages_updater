use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedConfig {
    pub locale: String,
    pub mine_path: String,
    pub mine_mode: String,
    pub upstream_path: String,
    pub upstream_mode: String,
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("packages_updater").join("config.json"))
}

pub fn load_config() -> SavedConfig {
    let path = match config_path() {
        Some(p) => p,
        None => return SavedConfig::default(),
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &SavedConfig) -> Result<()> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(()),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, json)?;
    Ok(())
}
