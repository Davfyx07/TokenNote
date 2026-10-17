use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub margin_top: i32,
    pub margin_right: i32,
    pub update_interval_secs: u64,
    pub auto_refresh_claude: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            margin_top: 24,
            margin_right: 24,
            update_interval_secs: 60,
            auto_refresh_claude: true,
        }
    }
}

impl WidgetConfig {
    fn get_config_path() -> Option<PathBuf> {
        let config_dir = dirs::config_dir()?.join("ai-usage-widget");
        Some(config_dir.join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path() {
            if path.exists() {
                if let Ok(file) = File::open(&path) {
                    let reader = BufReader::new(file);
                    if let Ok(cfg) = serde_json::from_reader(reader) {
                        return cfg;
                    }
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
        }
        Ok(())
    }
}
