use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub margin_top: i32,
    pub margin_right: i32,
    pub update_interval_secs: u64,
    pub auto_refresh_claude: bool,
    /// Providers agregados desde la ventana de configuración
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

/// Tipos de provider configurables: (kind, nombre para mostrar)
pub const PROVIDER_KINDS: [(&str, &str); 3] = [
    ("deepseek", "DeepSeek"),
    ("openrouter", "OpenRouter"),
    ("opencode", "OpenCode"),
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// "deepseek" | "openrouter" | "opencode"
    pub kind: String,
    #[serde(default)]
    pub api_key: String,
    /// deepseek: saldo de referencia en USD (100%); opencode: tokens por día; openrouter: no se usa
    #[serde(default)]
    pub budget: f64,
}

impl ProviderConfig {
    pub fn display_name(&self) -> &str {
        PROVIDER_KINDS
            .iter()
            .find(|(k, _)| *k == self.kind)
            .map_or(self.kind.as_str(), |(_, name)| name)
    }
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            margin_top: 24,
            margin_right: 24,
            update_interval_secs: 60,
            auto_refresh_claude: true,
            providers: Vec::new(),
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
                    match serde_json::from_reader(reader) {
                        Ok(cfg) => return cfg,
                        // No sobrescribir: el archivo puede tener API keys
                        Err(e) => {
                            tracing::warn!("config.json inválido, usando valores por defecto: {}", e);
                            return Self::default();
                        }
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
            let file = File::create(&path)?;
            // Contiene API keys: solo legible por el usuario
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
        }
        Ok(())
    }
}
